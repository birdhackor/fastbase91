#!/usr/bin/env python3
"""Fail-closed release-registry state checks."""

from __future__ import annotations

import argparse
import json
import string
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any

PYPI_BASE_URL = "https://pypi.org/pypi"
CRATES_BASE_URL = "https://crates.io/api/v1/crates"
USER_AGENT = "fastbase91-release-registry-check/1.0"
NOT_FOUND = object()
ERROR = 2


class RegistryStateError(RuntimeError):
    """Raised when registry state cannot be established safely."""


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    pypi_has = subparsers.add_parser("pypi-has")
    pypi_has.add_argument("package")
    pypi_has.add_argument("version")

    crates_has = subparsers.add_parser("crates-has")
    crates_has.add_argument("crate")
    crates_has.add_argument("version")

    pypi_matches = subparsers.add_parser("pypi-set-matches")
    pypi_matches.add_argument("package")
    pypi_matches.add_argument("version")
    pypi_matches.add_argument("manifest", type=Path)
    return parser.parse_args(argv)


def fetch_json(url: str) -> Any:
    request = urllib.request.Request(
        url,
        headers={"Accept": "application/json", "User-Agent": USER_AGENT},
    )
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            status = response.status
            if status != 200:
                raise RegistryStateError(f"GET {url} returned HTTP {status}")
            try:
                return json.load(response)
            except (UnicodeDecodeError, json.JSONDecodeError) as error:
                raise RegistryStateError(f"GET {url} returned invalid JSON") from error
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return NOT_FOUND
        raise RegistryStateError(f"GET {url} returned HTTP {error.code}") from error
    except (urllib.error.URLError, TimeoutError, OSError) as error:
        raise RegistryStateError(f"GET {url} failed: {error}") from error


def pypi_release(package: str, version: str) -> list[Any] | None:
    package_path = urllib.parse.quote(package, safe="")
    data = fetch_json(f"{PYPI_BASE_URL}/{package_path}/json")
    if data is NOT_FOUND:
        return None
    if not isinstance(data, dict) or not isinstance(data.get("releases"), dict):
        raise RegistryStateError("PyPI response does not contain a releases object")
    releases = data["releases"]
    if version not in releases:
        return None
    files = releases[version]
    if not isinstance(files, list):
        raise RegistryStateError(f"PyPI release {version!r} is not a file array")
    return files


def pypi_has(package: str, version: str) -> bool:
    return pypi_release(package, version) is not None


def crates_has(crate: str, version: str) -> bool:
    crate_path = urllib.parse.quote(crate, safe="")
    data = fetch_json(f"{CRATES_BASE_URL}/{crate_path}")
    if data is NOT_FOUND:
        return False
    if not isinstance(data, dict) or not isinstance(data.get("versions"), list):
        raise RegistryStateError("crates.io response does not contain a versions array")
    for index, record in enumerate(data["versions"]):
        if not isinstance(record, dict) or not isinstance(record.get("num"), str):
            raise RegistryStateError(
                f"crates.io version record {index} does not contain a string num"
            )
        if record["num"] == version:
            return True
    return False


def validate_sha256(value: Any, context: str) -> str:
    if not isinstance(value, str) or len(value) != 64:
        raise RegistryStateError(f"{context} has an invalid sha256")
    digest = value.lower()
    if any(character not in string.hexdigits for character in digest):
        raise RegistryStateError(f"{context} has an invalid sha256")
    return digest


def manifest_pypi_files(
    manifest_path: Path, package: str, version: str
) -> dict[str, str]:
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise RegistryStateError(f"manifest does not exist: {manifest_path}") from error
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise RegistryStateError(
            f"cannot read manifest {manifest_path}: {error}"
        ) from error

    if not isinstance(manifest, dict):
        raise RegistryStateError("manifest root must be an object")
    if manifest.get("package") != package:
        raise RegistryStateError(
            f"manifest package {manifest.get('package')!r} does not match {package!r}"
        )
    if manifest.get("version") != version:
        raise RegistryStateError(
            f"manifest version {manifest.get('version')!r} does not match {version!r}"
        )
    artifacts = manifest.get("artifacts")
    if not isinstance(artifacts, list):
        raise RegistryStateError("manifest artifacts must be an array")

    expected: dict[str, str] = {}
    for index, record in enumerate(artifacts):
        if not isinstance(record, dict):
            raise RegistryStateError(f"manifest artifact {index} must be an object")
        kind = record.get("kind")
        if kind == "crate":
            continue
        if kind not in {"wheel", "sdist"}:
            raise RegistryStateError(
                f"manifest artifact {index} has unsupported kind {kind!r}"
            )
        filename = record.get("filename")
        if (
            not isinstance(filename, str)
            or not filename
            or Path(filename).name != filename
        ):
            raise RegistryStateError(
                f"manifest artifact {index} has invalid filename {filename!r}"
            )
        if filename in expected:
            raise RegistryStateError(f"manifest repeats artifact filename {filename}")
        expected[filename] = validate_sha256(
            record.get("sha256"), f"manifest artifact {filename}"
        )
    if not expected:
        raise RegistryStateError("manifest contains no PyPI artifacts")
    return expected


def remote_pypi_files(files: list[Any], version: str) -> dict[str, str]:
    actual: dict[str, str] = {}
    for index, record in enumerate(files):
        if not isinstance(record, dict):
            raise RegistryStateError(f"PyPI release file {index} must be an object")
        filename = record.get("filename")
        digests = record.get("digests")
        if not isinstance(filename, str) or not filename:
            raise RegistryStateError(f"PyPI release file {index} has no filename")
        if filename in actual:
            raise RegistryStateError(f"PyPI repeats release filename {filename}")
        if not isinstance(digests, dict):
            raise RegistryStateError(f"PyPI release file {filename} has no digests")
        actual[filename] = validate_sha256(
            digests.get("sha256"), f"PyPI release file {filename}"
        )
    return actual


def pypi_set_matches(package: str, version: str, manifest_path: Path) -> bool:
    expected = manifest_pypi_files(manifest_path, package, version)
    release = pypi_release(package, version)
    if release is None:
        print(f"PyPI release is absent: {package} {version}", file=sys.stderr)
        return False
    actual = remote_pypi_files(release, version)

    missing = sorted(set(expected) - set(actual))
    unexpected = sorted(set(actual) - set(expected))
    mismatched = sorted(
        filename
        for filename in set(expected) & set(actual)
        if expected[filename] != actual[filename]
    )
    for filename in missing:
        print(f"PyPI set mismatch: missing {filename}", file=sys.stderr)
    for filename in unexpected:
        print(f"PyPI set mismatch: unexpected {filename}", file=sys.stderr)
    for filename in mismatched:
        print(
            f"PyPI set mismatch: sha256 differs for {filename}: "
            f"manifest={expected[filename]} registry={actual[filename]}",
            file=sys.stderr,
        )
    return not (missing or unexpected or mismatched)


def print_boolean(label: str, value: bool) -> int:
    print(f"{label}: {'true' if value else 'false'}")
    return 0 if value else 1


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        if args.command == "pypi-has":
            return print_boolean(
                f"PyPI has {args.package} {args.version}",
                pypi_has(args.package, args.version),
            )
        if args.command == "crates-has":
            return print_boolean(
                f"crates.io has {args.crate} {args.version}",
                crates_has(args.crate, args.version),
            )
        return print_boolean(
            f"PyPI set matches {args.package} {args.version}",
            pypi_set_matches(args.package, args.version, args.manifest),
        )
    except RegistryStateError as error:
        print(f"registry state error: {error}", file=sys.stderr)
        return ERROR


if __name__ == "__main__":
    raise SystemExit(main())
