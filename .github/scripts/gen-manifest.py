#!/usr/bin/env python3
"""Write the release-artifact manifest consumed by the release workflow."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--artifacts",
        required=True,
        type=Path,
        help="directory containing wheel and source-distribution artifacts",
    )
    parser.add_argument("--output", required=True, type=Path)
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


def main() -> int:
    args = parse_args()
    if not args.artifacts.is_dir():
        raise ValueError(f"artifact directory does not exist: {args.artifacts}")

    files = sorted(
        (path for path in args.artifacts.rglob("*") if path.is_file()),
        key=lambda path: path.name,
    )
    records: list[dict[str, str]] = []
    versions: set[str] = set()
    kinds: set[str] = set()
    filenames: set[str] = set()

    for path in files:
        kind = artifact_kind(path)
        if kind is None:
            continue
        if path.name in filenames:
            raise ValueError(f"duplicate artifact filename: {path.name}")
        filenames.add(path.name)
        versions.add(artifact_version(path, args.package, kind))
        kinds.add(kind)
        records.append(
            {
                "filename": path.name,
                "sha256": sha256(path),
                "kind": kind,
            }
        )

    if not records:
        raise ValueError("no wheel or source-distribution artifacts found")
    missing_kinds = {"wheel", "sdist"} - kinds
    if missing_kinds:
        raise ValueError(f"missing required artifact kinds: {', '.join(sorted(missing_kinds))}")
    if len(versions) != 1:
        raise ValueError(f"artifact versions do not agree: {', '.join(sorted(versions))}")

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
