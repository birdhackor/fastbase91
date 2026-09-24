# Benchmarks

These numbers were re-measured for the 0.1.2 release (see the [Changelog](changelog.md) for what changed). Throughput depends heavily on the machine and on whatever else it is doing, so read every number as **relative**, not as a guarantee. This is a single run on one laptop; run-to-run wobble of 10–15% is normal.

## Environment

- Apple M1, macOS 26.4.1
- CPython 3.13.15
- fastbase91 0.1.2 (from PyPI)
- pybase91 0.2.2 (from PyPI)

The run also confirms fastbase91, pybase91, and the pure-Python reference emit identical, interoperable basE91.

## Single-threaded throughput

Each table separates **encode** from **decode**, and decode is split into **lenient** (the default, which skips non-alphabet bytes) and **strict** (which rejects them). Strict decode is a fastbase91 / fastbase91-core feature; pybase91 0.2.2 and the pure-Python baseline have only one decode mode, shown as "—" under strict.

**1 KiB input**

| Implementation | encode (MiB/s) | decode lenient (MiB/s) | decode strict (MiB/s) |
| --- | ---: | ---: | ---: |
| fastbase91 | 1,152 | 837 | 437 |
| pybase91 | 754 | 621 | — |
| pure Python | 6.1 | 4.7 | — |

**64 KiB input**

| Implementation | encode (MiB/s) | decode lenient (MiB/s) | decode strict (MiB/s) |
| --- | ---: | ---: | ---: |
| fastbase91 | 1,375 | 948 | 469 |
| pybase91 | 975 | 827 | — |
| pure Python | 6.3 | 4.9 | — |

**1 MiB input**

| Implementation | encode (MiB/s) | decode lenient (MiB/s) | decode strict (MiB/s) |
| --- | ---: | ---: | ---: |
| fastbase91 | 1,624 | 1,076 | 488 |
| pybase91 | 1,046 | 850 | — |
| pure Python | 6.4 | 5.5 | — |

Strict decode checks every input byte against the alphabet, which is why it runs below lenient decode; it is still far above the pure-Python baseline. The pure-Python figures are GIL-bound Python bytecode.

## Concurrency scaling

**This table measures encode**, at 64 KiB, as aggregate MiB/s across threads, on GIL-enabled CPython 3.13. Both encode and decode release the GIL on inputs of 1,024 bytes or more, so decode scales the same way; encode is shown here as the representative case.

| Implementation | 1 thread (MiB/s) | 2 threads (MiB/s) | 4 threads (MiB/s) |
| --- | ---: | ---: | ---: |
| fastbase91 | 1,680 | 3,200 | 6,141 |
| pybase91 | 1,038 | 1,039 | 1,043 |
| pure Python | 7 | 7 | 7 |

fastbase91 releases the GIL on these 64 KiB calls, so Rust work from several threads overlaps and aggregate throughput grows with the thread count. An implementation that holds the GIL for the whole call stays flat, because its threads take turns; the pure-Python version is GIL-bound bytecode either way.
