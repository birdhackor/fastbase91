# Comparison

[正體中文](../zh/comparison.md)

All basE91 implementations in this comparison use the standard algorithm. fastbase91, pybase91, and the repository's pure-Python implementation produce byte-identical output and interoperate.

## basE91 implementations

| Option | Strengths | Trade-offs |
| --- | --- | --- |
| **fastbase91** | Free-threaded and Windows wheels; large one-shot calls release the GIL; reusable `no_std` Rust core; streaming and strict decoding; Rust core forbids unsafe code | Not the highest single-threaded throughput in this comparison |
| **pybase91** | Excellent single-threaded throughput from a Rust/PyO3 implementation | In the compared release, wheels target macOS/Linux and Python 3.11–3.13; no free-threaded or Windows wheel |
| **Pure Python** | No compiled dependency and the broadest source portability | Roughly 50–180 times slower than the Rust implementations in these measurements |

pybase91 is a strong choice when maximum single-threaded throughput is the priority on its supported Python and platform combinations. fastbase91 instead emphasizes free-threaded execution, cross-thread scaling for large calls, Windows distribution, a reusable `no_std` core, safe Rust, streaming, and strict decoding.

## Other binary-to-text encodings

| Encoding | Approximate size expansion | Notes |
| --- | ---: | --- |
| base64 | 33% | Widely supported; available in the Python standard library |
| base85 | 25% | Denser than base64; available in the Python standard library |
| basE91 | 23% | Densest of these three; its 91-character alphabet uses more punctuation |

The size figures are approximate and depend on input length. base64 and base85 are different formats, not alternate implementations of basE91, so their encoded data is not interchangeable with basE91.

Choose based on the surrounding protocol and runtime requirements: interoperability and ecosystem reach may matter more than density, while concurrency, platform wheels, or `no_std` support may determine the implementation.

