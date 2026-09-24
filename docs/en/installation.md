# Installation & compatibility

## Python

Install the package from PyPI:

```console
python -m pip install fastbase91
```

The package requires CPython 3.11 or later and supports CPython only.

### Pre-built wheels

| Platform and architecture | Regular CPython 3.11+ | Free-threaded CPython 3.14t | Free-threaded CPython 3.15t+ |
| --- | ---: | ---: | ---: |
| Linux x86_64 (glibc 2.28+) | ✓ | ✓ | ✓ |
| macOS arm64 (Apple silicon, macOS 11+) | ✓ | ✓ | ✓ |
| Windows x86_64 | ✓ | ✓ | ✓ |

Regular CPython uses an abi3 wheel; one wheel covers every version from 3.11 onward, and `pip` selects the matching wheel automatically.

### Other platforms

CPython on platforms such as Linux ARM64, Intel Mac, Windows ARM, Alpine/musl, and Linux with glibc older than 2.28 has no pre-built wheel. `pip` downloads the source distribution and builds it locally; this requires a Rust toolchain (stable, for example installed with rustup). CI validates source builds only on Linux x86_64.

### Unsupported environments

- PyPy and other non-CPython implementations are unsupported; there are no PyPI wheels for them, and CI does not build or test them.
- Free-threaded CPython 3.13t is unsupported: it has no wheel, and building from source fails. Free-threaded CPython requires 3.14t or later.
- Subinterpreters are unsupported: importing `fastbase91` in a subinterpreter created with `concurrent.interpreters`, for example, raises `ImportError`. Use it in the main interpreter.

### Free-threaded Python

Install with a free-threaded interpreter, such as `python3.14t`; `pip` selects the matching wheel:

```console
python3.14t -m pip install fastbase91
```

Check that the GIL remains disabled after import:

```console
python3.14t -c "import sys, fastbase91; print(sys._is_gil_enabled())"
```

This prints `False` when the GIL remains disabled; starting with `PYTHON_GIL=1` or `-X gil=1` enables it and prints `True`. See [Free-threading](free-threading.md) for details.

### Installed versions

```console
python -c "import fastbase91; print(fastbase91.__version__, fastbase91.CORE_VERSION)"
```

The two values are the Python package version and the linked Rust core version, respectively.

## Rust

Add the core crate:

```console
cargo add fastbase91-core
```

The minimum supported Rust version is 1.81.

### Features

| Feature configuration | Available APIs |
| --- | --- |
| `std` (default; includes `alloc`) | `encode`, `decode`, `encode_into`, `decode_into`, streaming `Encoder`/`Decoder`, `max_encoded_len`, and `max_decoded_len` |
| `alloc` only | `encode`, `decode`, `encode_into`, `decode_into`, streaming `Encoder`/`Decoder`, `max_encoded_len`, and `max_decoded_len` |
| Neither (`no_std`, no allocator) | `encode_into`, `decode_into`, streaming `Encoder`/`Decoder`, `max_encoded_len`, and `max_decoded_len` |

CI checks a bare-metal build for the `thumbv7em-none-eabihf` target. See [Usage](usage.md) for feature setup and Rust examples.

## Versions

The Python package `fastbase91` on PyPI and the Rust crate `fastbase91-core` on crates.io use independent version numbers. The core version bundled in a Python wheel is available as `CORE_VERSION`; see [Changelog](changelog.md) for version history.
