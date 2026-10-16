# Changelog

Every release produces byte-identical, interoperable standard basE91; upgrading never changes the encoded output. The Python package `fastbase91` and the Rust crate `fastbase91-core` are versioned independently; each release below shipped both packages at the version in its heading unless it says otherwise.

## 0.1.4

This release ships only the Python package; `fastbase91-core` is unchanged at 0.1.3.

- **Wheels for Linux ARM64** (glibc 2.28+) for regular CPython 3.11+ and free-threaded 3.14t and 3.15t+, so Docker on Apple silicon, AWS Graviton and other ARM64 Linux machines no longer build from source.
- **Wheels for Alpine and other musl-based Linux** (musl 1.2+) on x86_64 and ARM64, for regular CPython 3.11+. They include GCC's `libgcc_s`, which Rust needs on musl; its license is in `THIRD-PARTY-LICENSES`, which every distribution now ships.
- Encoding and decoding are unchanged, so the output is too. The full list of wheels is in [Installation & compatibility](installation.md).

## 0.1.3

- **Docstrings** for every public Python function, class and method, matching the type stub, so `help()` and editor hovers show the same text.
- **Rust API documentation** on docs.rs for the `alloc`-enabled `encode()` and `decode()`, plus a crate-level overview.
- **Package pages** on PyPI and crates.io now show the README and link to the documentation and the repository.
- Wheels are built with PyO3 0.29.3. Encoding and decoding are unchanged, so the output is too; `no_std`, MSRV 1.81 and `#![forbid(unsafe_code)]` are all preserved.

## 0.1.2

- **Encode** rewritten to a word-at-a-time hot loop backed by a pair lookup table.
- **Lenient decode** gained an 8-byte block fast path, which re-engages as soon as a pending symbol is consumed — so line-wrapped and streamed input benefit too.
- Output is unchanged from earlier releases; `no_std`, MSRV 1.81 and `#![forbid(unsafe_code)]` are all preserved. See [Benchmarks](benchmarks.md).

## 0.1.1

- **Encode** hot loop straightened out.
- **Decode** split into separate lenient (default) and strict paths, which sped up the default path.

## 0.1.0

- Initial release: the Python package `fastbase91` and the `no_std` Rust crate `fastbase91-core`.
