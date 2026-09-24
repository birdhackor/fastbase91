# Overview

[正體中文](../zh/index.md)

fastbase91 provides standard basE91 encoding through a safe Rust core and a Python extension. basE91 represents binary data with 91 printable ASCII characters. Its output is about 23% larger than the input, compared with about 33% for base64.

## Packages

| Package | Purpose |
| --- | --- |
| `fastbase91` | Python package backed by PyO3 and the Rust core |
| `fastbase91-core` | Reusable Rust crate with `no_std` support |

Both packages use the same codec. fastbase91, the repository's pure-Python reference implementation, and pybase91 produce byte-for-byte identical output and can decode one another's output.

## Design focus

- **Portability:** Python wheels include Windows, macOS, and Linux targets; the Rust core can run without `std` or an allocator.
- **Concurrency:** large one-shot Python calls release the GIL, and dedicated free-threaded wheels are published.
- **Safety:** the Rust core uses `#![forbid(unsafe_code)]`.
- **Control:** both APIs offer streaming operation, and decoding can reject bytes outside the basE91 alphabet.

fastbase91 is not positioned as the fastest single-threaded implementation. Its emphasis is a balanced interface across free-threading, portability, safety, streaming, and strict decoding. See [Comparison](comparison.md) and [Benchmarks](benchmarks.md) for the measured trade-offs.

## Quick start

```python
import fastbase91

encoded = fastbase91.encode(b"hello")
assert encoded == b"TPwJh>A"
assert fastbase91.decode(encoded) == b"hello"
```

Continue with [Usage](usage.md) for the complete Python and Rust APIs.

