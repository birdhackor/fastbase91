# Benchmarks

These numbers were re-measured for the 0.1.1 release, which includes a straight-line encode hot loop and a split decode path (see below). Throughput depends heavily on the machine and on whatever else it is doing, so read every number as **relative**, not as a guarantee. This is a single run on one laptop; run-to-run wobble of 10–15% is normal.

## Environment

- Apple M1, macOS 26.4.1
- CPython 3.13.15
- fastbase91 0.1.1 (built from source in this repository)
- pybase91 0.2.2 (from PyPI)

Every figure is MiB/s of **input** bytes. The run also confirms fastbase91, pybase91, and the pure-Python reference emit identical, interoperable basE91.

## Single-threaded throughput

Each table separates **encode** from **decode**, and decode is split into **lenient** (the default, which skips non-alphabet bytes) and **strict** (which rejects them). Strict decode is a fastbase91 / fastbase91-core feature; pybase91 0.2.2 and the pure-Python baseline have only one decode mode, shown as "—" under strict.

**1 KiB input**

| Implementation | encode | decode (lenient) | decode (strict) |
| --- | ---: | ---: | ---: |
| fastbase91 (Rust) | 614 | 683 | 480 |
| pybase91 (Rust) | 890 | 737 | — |
| pure Python | 7.0 | 5.4 | — |

**64 KiB input**

| Implementation | encode | decode (lenient) | decode (strict) |
| --- | ---: | ---: | ---: |
| fastbase91 (Rust) | 715 | 856 | 565 |
| pybase91 (Rust) | 1,043 | 827 | — |
| pure Python | 7.4 | 5.4 | — |

**1 MiB input**

| Implementation | encode | decode (lenient) | decode (strict) |
| --- | ---: | ---: | ---: |
| fastbase91 (Rust) | 743 | 863 | 572 |
| pybase91 (Rust) | 1,048 | 883 | — |
| pure Python | 7.4 | 5.5 | — |

Reading these: pybase91 encodes faster on one thread (roughly 1.4–1.5x here). Lenient decode is now a near tie — fastbase91 edges ahead at 64 KiB, pybase91 edges ahead at 1 MiB. Strict decode costs fastbase91 roughly a third of its lenient decode speed, the price of checking every byte, and is still about 100x the pure-Python baseline.

## Concurrency scaling

**This table measures encode**, at 64 KiB, as aggregate MiB/s across threads, on GIL-enabled CPython 3.13. Both encode and decode release the GIL on inputs of 1,024 bytes or more, so decode scales the same way; encode is shown here as the representative case.

| Implementation | 1 thread | 2 threads | 4 threads |
| --- | ---: | ---: | ---: |
| fastbase91 (Rust) | 733 | 1,407 | 2,704 |
| pybase91 (Rust) | 1,039 | 1,040 | 1,040 |
| pure Python | 7 | 7 | 7 |

fastbase91 releases the GIL on these 64 KiB calls, so Rust work from several threads overlaps and total throughput climbs almost linearly. It starts behind pybase91 on one thread, passes it at two, and is far ahead at four. pybase91 stays flat because it holds the GIL for the call; the pure-Python version is stuck on GIL-bound bytecode either way.

The takeaway is a trade-off, not a ranking: pybase91 wins one-thread encode, while fastbase91 wins as soon as the workload has a few large calls to run at once.
