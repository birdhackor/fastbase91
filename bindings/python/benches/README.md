# P7 Python full-path measurements

`p7_measure.py` is a standard-library-only harness for the P7 measurement
pass. It measures the installed extension, rather than an import from the
source tree. Build the intended release variant first, then run the harness
with that same virtual environment:

```sh
cd bindings/python
UV_CACHE_DIR=/Users/yclin/project/fastbase91/.uv-cache .venv/bin/maturin develop --release
.venv/bin/python benches/p7_measure.py run --variant shipping --output /private/tmp/fastbase91-p7-shipping.json

UV_CACHE_DIR=/Users/yclin/project/fastbase91/.uv-cache .venv/bin/maturin develop --release --features bench_no_detach
.venv/bin/python benches/p7_measure.py run --variant bench_no_detach --skip-rss --skip-scaling --output /private/tmp/fastbase91-p7-no-detach.json

cd ../..
bindings/python/.venv/bin/python bindings/python/benches/p7_measure.py core --output /private/tmp/fastbase91-p7-core.json
bindings/python/.venv/bin/python bindings/python/benches/p7_measure.py render --input /private/tmp/fastbase91-p7-shipping.json
bindings/python/.venv/bin/python bindings/python/benches/p7_measure.py compare --shipping /private/tmp/fastbase91-p7-shipping.json --no-detach /private/tmp/fastbase91-p7-no-detach.json --core /private/tmp/fastbase91-p7-core.json
```

The release `shipping` build is the normal binding: one-shot `encode` and
`decode` release the GIL via `Python::detach` only when the input is at least
`ONESHOT_DETACH_THRESHOLD` (1 KiB); smaller one-shots keep the GIL and skip the
detach overhead. `bench_no_detach` is an opt-in, benchmark-only Cargo feature
that disables detach entirely for every one-shot; it must not be used for a
shipping wheel. Diffing a detaching build against `bench_no_detach` isolates the
per-call detach/reattach cost. The P7 pass established that cost against an
always-detach build (before the threshold constant existed); to re-validate the
threshold on another machine, temporarily set `ONESHOT_DETACH_THRESHOLD` to 0 so
every size detaches, then diff against `bench_no_detach`.

The harness writes raw JSON, including every individual `perf_counter_ns`
sample. The `render` subcommand prints compact tables, while `compare` prints
the detach delta beside the common 1 KiB and 64 KiB points emitted by the
unchanged core benchmark, plus a 1 MiB timing decomposition. The existing core
benchmark does not emit other small-payload sizes, so the comparison labels
those cells `NA` rather than interpolating them.

## Protocol

- Build profile is always `release`; it is recorded in the raw result.
- Full-path throughput covers 1 KiB, 64 KiB, and 1 MiB inputs for deterministic
  random bytes, all-zero bytes, all-`0xff` bytes, and a repeated UTF-8 text
  sample. It warms up 3 calls and records 13 one-call samples, reporting their
  median. Encode throughput uses input bytes as its denominator; decode uses
  decoded/output bytes as its denominator.
- Small-payload latency uses the deterministic random sample at 0, 8, 64, 256,
  1 KiB, 4 KiB, 16 KiB, and 64 KiB. It warms up 9 calls and records 31 one-call
  samples, reporting nanoseconds per call from `time.perf_counter_ns`.
- Timed calls include the binding's input borrowing/copying, Rust computation,
  output `Vec` allocation and zeroing, `Vec`-to-`bytes` copy, and Python object
  creation. Payload construction, result comparison, warmups, and JSON output
  occur outside each timed interval. Timing inputs are immutable `bytes`, so
  the normal borrowed input path is measured.
- Peak RSS uses a fresh child process for each condition, creates a 64 MiB
  all-zero `bytes` or directly-created `bytearray`, does exactly one one-shot
  encode, and records `resource.getrusage(RUSAGE_SELF).ru_maxrss`. On macOS
  that value is bytes; on Linux it is KiB and the JSON also gives a normalized
  byte value. The `bytearray` constructor is direct so it does not make a
  temporary same-sized `bytes` object in the child.
- Scaling uses a persistent `ThreadPoolExecutor` with 1, 2, 4, and 8 workers.
  Every worker gets a distinct 1 MiB deterministic payload; every measured
  batch validates every returned byte string against a separately prepared
  one-shot result. There are 3 warmup batches and 11 measured batches. Total
  throughput uses total input bytes for encode and total decoded/output bytes
  for decode.

The core command executes the repository's unchanged
`cargo bench -p fastbase91-core` benchmark. Its `*-slice` cases preallocate
output and are a useful inner-loop proxy; its `*-one-shot` cases include the
core output `Vec` allocation. Neither is a profiler attribution, and neither
includes Python boundary work.
