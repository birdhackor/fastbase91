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
2. **Single-threaded throughput** — encode, decode-lenient, and decode-strict
   MiB/s at 1 KiB, 64 KiB, 1 MiB. Strict decode (reject non-alphabet bytes) is a
   fastbase91 feature; pybase91 0.2.2 and the pure-Python baseline have only one
   decode mode, so their strict column is `-`.
3. **Concurrency scaling** — aggregate encode throughput at 1/2/4 threads.
   fastbase91 releases the GIL on inputs ≥ 1 KiB (encode and decode alike), so
   its Rust compute can overlap across threads.

## Sample results

Numbers are **machine-dependent**; treat them as relative, not absolute, and
expect 10–15% run-to-run wobble. The snapshot below is a single run on Apple M1,
macOS 26.4.1, CPython 3.13.15 (fastbase91 0.1.2 and pybase91 0.2.2 from PyPI).

Single-threaded throughput (MiB/s of input), encode / decode-lenient / decode-strict:

| impl        | 1 KiB enc/len/strict | 64 KiB enc/len/strict | 1 MiB enc/len/strict |
| ----------- | -------------------- | --------------------- | -------------------- |
| fastbase91  | 1152 / 837 / 437     | 1375 / 948 / 469      | 1624 / 1076 / 488    |
| pybase91    | 754 / 621 / -        | 975 / 827 / -         | 1046 / 850 / -       |
| pure-Python | 6.1 / 4.7 / -        | 6.3 / 4.9 / -         | 6.4 / 5.5 / -        |

Concurrency scaling (64 KiB **encode**, aggregate MiB/s, GIL-enabled CPython 3.13):

| impl        | 1 thread | 2 threads | 4 threads |
| ----------- | -------- | --------- | --------- |
| fastbase91  | 1680     | 3200      | 6141      |
| pybase91    | 1038     | 1039      | 1043      |
| pure-Python | 7        | 7         | 7         |

The single-threaded tables and the concurrency table measure different things.
fastbase91 releases the GIL on calls ≥ 1 KiB, so its aggregate throughput grows
with the thread count, while an implementation that holds the GIL for the call
stays flat. Strict decode checks every byte against the alphabet, so it runs
below lenient decode. fastbase91 also ships free-threaded (3.14t/3.15t) and
Windows wheels, which pybase91 0.2.2 does not.
