# Overview

fastbase91 is a basE91 encoder for Python and Rust, built for the things people actually get stuck on: running in parallel, running on free-threaded Python, running on Windows, and running with no operating system at all.

basE91 is a way to pack binary data into 91 printable ASCII characters. The encoded text is about 23% bigger than the raw bytes, versus about 33% for base64 — so it is a tighter fit when you have to move binary through a text-only channel.

## Quick start

```console
python -m pip install fastbase91
```

```python
import fastbase91

encoded = fastbase91.encode(b"hello")
assert encoded == b"TPwJh>A"
assert fastbase91.decode(encoded) == b"hello"
```

For Rust, run `cargo add fastbase91-core`; see [Usage](usage.md).

## What you get

| Package | What it is |
| --- | --- |
| `fastbase91` | The Python package (a compiled extension built on PyO3 and the Rust core) |
| `fastbase91-core` | The Rust crate on its own, with `no_std` support for bare-metal targets |

fastbase91 implements standard [basE91](https://base91.sourceforge.net/). Its output is byte-for-byte identical to the original reference implementation — the core crate's tests compare against that C code — so it interoperates with any other standard basE91 implementation. To check one yourself: every standard implementation encodes `hello` as `TPwJh>A`, the value in the quick start above.

## What it is built for

- **Concurrency.** A large one-shot call hands the work to Rust and lets go of Python's global lock (the GIL), so several calls on several threads run at the same time instead of taking turns. See [Benchmarks](benchmarks.md).
- **Free-threaded Python.** The extension is marked as safe for the no-GIL build, so importing it does not switch the GIL back on. [Why that matters](free-threading.md).
- **Portability.** Pre-built Python wheels cover Linux (x86_64), macOS (Apple silicon), and Windows (x86_64); on other CPython platforms, pip tries to build from source, which needs a Rust toolchain (see [Installation & compatibility](installation.md)). The Rust core runs without `std` or even a memory allocator.
- **Safety.** The Rust core is `#![forbid(unsafe_code)]` — the compiler rejects any unsafe block.
- **Control.** Both APIs can stream (encode/decode in chunks), and decoding can reject anything outside the basE91 alphabet instead of silently skipping it.

Next: examples and explanations are in [Usage](usage.md); the full API is in [API reference](api-reference.md).
