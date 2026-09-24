# fastbase91

[![crates.io](https://img.shields.io/crates/v/fastbase91-core.svg)](https://crates.io/crates/fastbase91-core)
[![PyPI](https://img.shields.io/pypi/v/fastbase91.svg)](https://pypi.org/project/fastbase91/)
[![Rust CI](https://github.com/birdhackor/fastbase91/actions/workflows/rust.yml/badge.svg)](https://github.com/birdhackor/fastbase91/actions/workflows/rust.yml)
[![Python CI](https://github.com/birdhackor/fastbase91/actions/workflows/python.yml/badge.svg)](https://github.com/birdhackor/fastbase91/actions/workflows/python.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

A fast [basE91](https://base91.sourceforge.net/) binary-to-text codec: a
`no_std`, `#![forbid(unsafe_code)]` Rust core (`fastbase91-core`) with a
PyO3-based Python binding (`fastbase91`).

basE91 packs binary data into 91 printable ASCII characters, giving roughly
23% size overhead — noticeably tighter than base64's ~33%.

## Features

- **Fast** — a tight Rust core; the Python binding releases the GIL on larger
  inputs so calls can overlap.
- **Safe** — `#![forbid(unsafe_code)]` throughout the core.
- **Portable** — `no_std` core (optional `alloc`/`std`); runs on bare metal.
- **Streaming** — incremental `Encoder`/`Decoder` for data that does not fit in
  memory at once.
- **Free-threaded ready** — the Python extension declares `gil_used = false`
  and ships free-threaded wheels (CPython 3.14t / 3.15t).
- **One wheel per platform** — an abi3 wheel covers CPython 3.11+.

## Install

Rust:

```sh
cargo add fastbase91-core
```

Python:

```sh
pip install fastbase91      # or: uv add fastbase91
```

## Usage

Rust:

```rust
use fastbase91_core::{encode, decode, DecodeOptions};

let encoded = encode(b"hello").unwrap();  // -> b"TPwJh>A"
let decoded = decode(&encoded, DecodeOptions::default()).unwrap();
assert_eq!(decoded, b"hello");
```

Python:

```python
import fastbase91

encoded = fastbase91.encode(b"hello")     # -> b"TPwJh>A"
assert fastbase91.decode(encoded) == b"hello"
```

## Documentation

Full documentation (English / 正體中文), including usage, the free-threading
support matrix, benchmarks, and a changelog:
**https://birdhackor.github.io/fastbase91/**

## Versioning

The Rust core and the Python package are released on two independent version
lines: `fastbase91-core` on [crates.io](https://crates.io/crates/fastbase91-core)
via `fastbase91-core-vX.Y.Z` tags, and `fastbase91` wheels on
[PyPI](https://pypi.org/project/fastbase91/) via `vX.Y.Z` tags. See
[MAINTENANCE.md](MAINTENANCE.md) for the release runbook.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at
your option.
