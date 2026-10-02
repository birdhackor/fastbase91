# Changelog

Every release produces byte-identical, interoperable standard basE91; upgrading never changes the encoded output. The Python package `fastbase91` and the Rust crate `fastbase91-core` are versioned independently; each release below shipped both packages at the version in its heading.

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
