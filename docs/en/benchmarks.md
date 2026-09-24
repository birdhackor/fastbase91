# Benchmarks

[正體中文](../zh/benchmarks.md)

These are the repository's recorded sample results, not fresh measurements. Throughput depends on the machine and runtime, so use the numbers for relative comparison rather than as absolute guarantees.

## Environment

- Apple Silicon macOS
- CPython 3.13
- fastbase91 0.1.0
- pybase91 0.2.2

All throughput is measured in MiB/s of input. The correctness check also confirms that fastbase91, pybase91, and the pure-Python implementation emit identical, interoperable basE91 data.

## Single-threaded throughput

| Implementation | 1 KiB encode/decode | 64 KiB encode/decode | 1 MiB encode/decode |
| --- | ---: | ---: | ---: |
| fastbase91 (Rust) | 402 / 424 | 450 / 473 | 453 / 475 |
| pybase91 (Rust) | 903 / 754 | 1,038 / 876 | 1,049 / 875 |
| pure Python | 7.3 / 5.3 | 7.2 / 5.4 | 7.2 / 5.3 |

pybase91 has the higher single-threaded throughput in every row of this sample.

## Concurrency scaling

64 KiB encoding, aggregate MiB/s, on GIL-enabled CPython 3.13:

| Implementation | 1 thread | 2 threads | 4 threads |
| --- | ---: | ---: | ---: |
| fastbase91 (Rust) | 452 | 872 | 1,675 |
| pybase91 (Rust) | 946 | 913 | 954 |
| pure Python | 7 | 7 | 7 |

fastbase91 releases the GIL for these 64 KiB one-shot calls, so Rust computation from multiple threads overlaps and aggregate throughput grows. It overtakes pybase91 in the four-thread result. pybase91 remains approximately flat because the measured call holds the GIL, and the pure-Python implementation is GIL-bound bytecode.

The result is a trade-off, not a universal ranking: pybase91 leads for single-threaded work in this environment, while fastbase91 benefits when a workload can run several sufficiently large calls concurrently.
