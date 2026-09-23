# Encoder fuzzing

The `encode` target feeds arbitrary byte slices to the allocating, slice, and
streaming encoder APIs. Any panic is a crash; it also asserts that all three API
forms produce identical bytes. C-oracle differential coverage lives in the
integration tests so the fuzz loop does not spawn one process per input.

Reproduce the bounded smoke run with:

```console
cargo +nightly fuzz run encode --fuzz-dir fuzz -- -max_total_time=60 -timeout=5
```

The run budget is 60 seconds with a 5-second per-input timeout. Interesting
inputs are retained in `fuzz/corpus/encode/`; crashes are retained in
`fuzz/artifacts/encode/`. Do not delete a crash until it has become a regression
test.
