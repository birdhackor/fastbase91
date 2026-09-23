#!/usr/bin/env python3
"""Write the release-artifact manifest consumed by the release workflow."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--artifacts",
        required=True,
        type=Path,
        help="directory containing wheel and source-distribution artifacts",
    )
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument(
        "--matrix",
        required=True,
        type=Path,
        help="wheel policy matrix used by the build and test jobs",
    )
    parser.add_argument("--package", required=True)
    parser.add_argument("--source-commit", required=True)
    return parser.parse_args()


def artifact_kind(path: Path) -> str | None:
    if path.name.endswith(".whl"):
        return "wheel"
    if path.name.endswith(".tar.gz"):
        return "sdist"
    return None


def artifact_version(path: Path, package: str, kind: str) -> str:
    prefix = f"{package}-"
    if not path.name.startswith(prefix):
        raise ValueError(f"{path.name}: expected package prefix {prefix!r}")

    if kind == "wheel":
        remainder = path.name[len(prefix) : -len(".whl")]
        try:
            version, _python_tag, _abi_tag, _platform_tag = remainder.split("-", 3)
        except ValueError as error:
            raise ValueError(f"{path.name}: not a valid wheel filename") from error
    else:
        version = path.name[len(prefix) : -len(".tar.gz")]
        if not version:
            raise ValueError(f"{path.name}: source distribution has no version")
    return version


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as artifact:
        for chunk in iter(lambda: artifact.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_wheel_matrix(path: Path) -> dict[str, dict[str, Any]]:
    try:
        matrix = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise ValueError(f"wheel matrix does not exist: {path}") from error
    except json.JSONDecodeError as error:
        raise ValueError(f"wheel matrix is not valid JSON: {error}") from error

    if not isinstance(matrix, dict) or set(matrix) != {"include"}:
        raise ValueError("wheel matrix root must contain only an include array")
    if not isinstance(matrix["include"], list) or not matrix["include"]:
        raise ValueError("wheel matrix include must be a non-empty array")

    entries: dict[str, dict[str, Any]] = {}
    required = {"id", "python_tag", "abi_tag", "platform_tag_regex"}
    for index, entry in enumerate(matrix["include"]):
        if not isinstance(entry, dict):
            raise ValueError(f"wheel matrix entry {index} must be an object")
        missing = required - entry.keys()
        if missing:
            raise ValueError(
                f"wheel matrix entry {index} is missing: {', '.join(sorted(missing))}"
            )
        artifact_id = entry["id"]
        if not isinstance(artifact_id, str) or not re.fullmatch(
            r"[a-z0-9][a-z0-9-]*", artifact_id
        ):
            raise ValueError(
                f"wheel matrix entry {index} has invalid id {artifact_id!r}"
            )
        if artifact_id in entries:
            raise ValueError(f"wheel matrix repeats id: {artifact_id}")
        for key in ("python_tag", "abi_tag", "platform_tag_regex"):
            if not isinstance(entry[key], str) or not entry[key]:
                raise ValueError(f"wheel matrix entry {artifact_id} has invalid {key}")
        try:
            re.compile(entry["platform_tag_regex"])
        except re.error as error:
            raise ValueError(
                f"wheel matrix entry {artifact_id} has invalid platform_tag_regex: {error}"
            ) from error
        entries[artifact_id] = entry
    return entries


def wheel_tags(path: Path) -> tuple[str, str, str]:
    try:
        _prefix, python_tag, abi_tag, platform_tag = path.name[: -len(".whl")].rsplit(
            "-", 3
        )
    except ValueError as error:
        raise ValueError(f"{path.name}: not a valid wheel filename") from error
    return python_tag, abi_tag, platform_tag


def collect_policy_artifacts(
    artifacts: Path, matrix_path: Path
) -> list[tuple[Path, str]]:
    entries = read_wheel_matrix(matrix_path)
    expected_directories = {f"wheels-{artifact_id}" for artifact_id in entries}
    actual_directories: set[str] = set()
    for path in artifacts.iterdir():
        if path.name.startswith("wheels-"):
            if not path.is_dir():
                raise ValueError(f"wheel artifact path is not a directory: {path.name}")
            actual_directories.add(path.name)

    missing_directories = sorted(expected_directories - actual_directories)
    unexpected_directories = sorted(actual_directories - expected_directories)
    if missing_directories:
        raise ValueError(
            "missing expected wheel artifact directories: "
            + ", ".join(missing_directories)
        )
    if unexpected_directories:
        raise ValueError(
            "unexpected wheel artifact directories: "
            + ", ".join(unexpected_directories)
        )

    selected: list[tuple[Path, str]] = []
    filenames: set[str] = set()
    for artifact_id, entry in entries.items():
        directory = artifacts / f"wheels-{artifact_id}"
        wheels = sorted(path for path in directory.rglob("*.whl") if path.is_file())
        if len(wheels) != 1:
            raise ValueError(
                f"wheels-{artifact_id} must contain exactly one wheel, found {len(wheels)}"
            )
        wheel = wheels[0]
        if wheel.name in filenames:
            raise ValueError(f"duplicate artifact filename: {wheel.name}")
        filenames.add(wheel.name)

        python_tag, abi_tag, platform_tag = wheel_tags(wheel)
        if python_tag != entry["python_tag"] or abi_tag != entry["abi_tag"]:
            raise ValueError(
                f"wheels-{artifact_id} contains {wheel.name} with tag "
                f"{python_tag}-{abi_tag}; expected "
                f"{entry['python_tag']}-{entry['abi_tag']}"
            )
        if re.fullmatch(entry["platform_tag_regex"], platform_tag) is None:
            raise ValueError(
                f"wheels-{artifact_id} contains {wheel.name} with platform tag "
                f"{platform_tag!r}; expected /{entry['platform_tag_regex']}/"
            )
        selected.append((wheel, "wheel"))

    sdist_directory = artifacts / "sdist"
    if not sdist_directory.is_dir():
        raise ValueError("missing expected sdist artifact directory: sdist")
    sdists = sorted(
        path for path in sdist_directory.rglob("*.tar.gz") if path.is_file()
    )
    if len(sdists) != 1:
        raise ValueError(
            f"sdist must contain exactly one source distribution, found {len(sdists)}"
        )
    if sdists[0].name in filenames:
        raise ValueError(f"duplicate artifact filename: {sdists[0].name}")
    filenames.add(sdists[0].name)
    selected.append((sdists[0], "sdist"))

    discovered = {
        path
        for path in artifacts.rglob("*")
        if path.is_file() and artifact_kind(path) is not None
    }
    selected_paths = {path for path, _kind in selected}
    unexpected_files = sorted(discovered - selected_paths, key=lambda path: str(path))
    if unexpected_files:
        raise ValueError(
            "unexpected wheel or source-distribution artifacts: "
            + ", ".join(str(path.relative_to(artifacts)) for path in unexpected_files)
        )
    return selected


def main() -> int:
    args = parse_args()
    if not args.artifacts.is_dir():
        raise ValueError(f"artifact directory does not exist: {args.artifacts}")

    files = collect_policy_artifacts(args.artifacts, args.matrix)
    records: list[dict[str, str]] = []
    versions: set[str] = set()

    for path, kind in files:
        versions.add(artifact_version(path, args.package, kind))
        records.append(
            {
                "filename": path.name,
                "sha256": sha256(path),
                "kind": kind,
            }
        )

    if len(versions) != 1:
        raise ValueError(
            f"artifact versions do not agree: {', '.join(sorted(versions))}"
        )

    manifest = {
        "package": args.package,
        "version": versions.pop(),
        "source_commit": args.source_commit,
        "artifacts": records,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {args.output} for {len(records)} artifacts")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ValueError as error:
        print(f"manifest error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
