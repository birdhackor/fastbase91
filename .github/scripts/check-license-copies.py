#!/usr/bin/env python3
"""Fail when shipped license copies drift from the repository-root originals."""

from __future__ import annotations

from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
LICENSES = ("LICENSE-MIT", "LICENSE-APACHE")
COPIES = (ROOT / "bindings/python", ROOT / "crates/fastbase91-core")


def display(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def main() -> int:
    differences = 0
    for license_name in LICENSES:
        source = ROOT / license_name
        source_bytes = source.read_bytes()
        for directory in COPIES:
            candidate = directory / license_name
            if candidate.read_bytes() != source_bytes:
                print(f"license drift: {display(candidate)} differs from {display(source)}")
                differences += 1
    if differences:
        print(f"license copy check failed: {differences} differing file(s)")
        return 1
    print("license copies match byte-for-byte (6 files)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
