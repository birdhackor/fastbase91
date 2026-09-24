# Changelog

Every release produces byte-identical, interoperable standard basE91; upgrading never changes the encoded output. The Python package `fastbase91` and the Rust crate `fastbase91-core` share a version number.

## 0.1.2

- **Encode** rewritten to a word-at-a-time hot loop backed by a pair lookup table.
- **Lenient decode** gained an 8-byte block fast path, which re-engages as soon as a pending symbol is consumed — so line-wrapped and streamed input benefit too.
- Output is unchanged from earlier releases; `no_std`, MSRV 1.81 and `#![forbid(unsafe_code)]` are all preserved. See [Benchmarks](benchmarks.md).

## 0.1.1

- **Encode** hot loop straightened out.
- **Decode** split into separate lenient (default) and strict paths, which sped up the default path.

## 0.1.0

- Initial release: the Python package `fastbase91` and the `no_std` Rust crate `fastbase91-core`.
