# Benchmarks

A cross-implementation basE91 comparison:

- **fastbase91** — this project (Rust core + PyO3 binding).
- **pure-Python** (`purepy_base91.py`) — a straightforward pure-Python basE91,
  vendored verbatim from the author's own project to serve as a baseline.
- **pybase91** — a third-party Rust/PyO3 basE91 extension on PyPI.

All three implement standard basE91 and produce byte-identical, interoperable
output.

## Running

```sh
uv venv .venv --python 3.13
uv pip install --python .venv/bin/python fastbase91 pybase91
.venv/bin/python run_bench.py
```

## What it measures

1. **Correctness / interop** — round-trips `bytes(range(256))` and checks all
   three encoders emit identical text.
2. **Single-threaded throughput** — encode/decode MiB/s at 1 KiB, 64 KiB, 1 MiB.
3. **Concurrency scaling** — aggregate encode throughput at 1/2/4 threads.
   fastbase91 releases the GIL on inputs ≥ 1 KiB, so its Rust compute can
   overlap across threads.

## Sample results

Numbers are **machine-dependent**; treat them as relative, not absolute. The
snapshot below is CPython 3.13 on Apple Silicon macOS
(fastbase91 0.1.0, pybase91 0.2.2).

Single-threaded throughput (MiB/s of input):

| impl              | 1 KiB enc/dec | 64 KiB enc/dec | 1 MiB enc/dec |
| ----------------- | ------------- | -------------- | ------------- |
| fastbase91 (Rust) | 402 / 424     | 450 / 473      | 453 / 475     |
| pybase91 (Rust)   | 903 / 754     | 1038 / 876     | 1049 / 875    |
| pure-Python       | 7.3 / 5.3     | 7.2 / 5.4      | 7.2 / 5.3     |

Concurrency scaling (64 KiB encode, aggregate MiB/s, GIL-enabled CPython 3.13):

| impl              | 1 thread | 2 threads | 4 threads |
| ----------------- | -------- | --------- | --------- |
| fastbase91 (Rust) | 452      | 872       | 1675      |
| pybase91 (Rust)   | 946      | 913       | 954       |
| pure-Python       | 7        | 7         | 7         |

Reading these together: **pybase91 has the higher single-threaded throughput**,
while **fastbase91 scales with threads** (it releases the GIL, so parallel calls
overlap) and overtakes pybase91 once a couple of threads are in play. pybase91
stays flat because it holds the GIL for the call; pure-Python is GIL-bound
bytecode. fastbase91 also ships free-threaded (3.13t/3.15t) and Windows wheels,
which pybase91 currently does not.
