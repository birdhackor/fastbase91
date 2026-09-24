# Comparison

**The short version:** if you want the fastest single encode on one thread, use `pybase91`. If you want to run many encodes/decodes in parallel, run on free-threaded Python or Windows, embed a `no_std` core, or reject malformed input, use fastbase91. They produce identical bytes and decode each other's output, so you can even use both.

Every basE91 implementation here uses the standard algorithm. fastbase91, `pybase91`, and this repository's pure-Python reference are byte-for-byte interchangeable.

## Where pybase91 is faster, and why

On a single thread, `pybase91` (the `base91-rs` crate) encodes faster than fastbase91, and we are not going to pretend otherwise. The reason is specific and worth understanding, because it is easy to misread as "it uses SIMD and we don't."

`pybase91`'s interoperable encoder — the one plain `pybase91.encode()` calls — is **hand-tuned scalar code**: ordinary one-value-at-a-time Rust, carefully arranged so the LLVM compiler turns it into tight machine code. That hand-tuning is where its single-threaded edge comes from.

It *also* ships SIMD routines (processing many bytes per instruction), but those implement a **different, non-interoperable** encoding. Standard `pybase91.encode()` does not use them, and their output is not basE91 you can hand to another library. So the fair comparison — the interoperable path everyone actually uses — is scalar against scalar, and there `pybase91`'s hand-tuning simply wins on one thread.

## Where fastbase91 is built to win

- **Concurrency.** On a call of 1,024 bytes or more, fastbase91 releases Python's global lock (the GIL) with `py.detach` and does the work in Rust, so calls on different threads overlap. `pybase91` holds the GIL for the whole call, so extra threads wait their turn — its single-threaded lead does not turn into concurrent throughput. Past a couple of threads, fastbase91's aggregate throughput overtakes it (see [Benchmarks](benchmarks.md)).
- **No copy on the common input.** When you pass `bytes`, fastbase91 borrows the buffer directly (`PyBackedBytes`) instead of copying it. That borrow is safe to hold while the GIL is released precisely because `bytes` is immutable — nothing can change it mid-call. (A writable `bytearray` or `memoryview` is copied first, since it *could* change.)
- **Decode is no longer the weak spot.** fastbase91 now runs lenient and strict decoding down separate paths. Splitting them let the default (lenient) path speed up a lot — in the current measurements it lands roughly on par with `pybase91`'s decode. Strict decoding, which checks every byte against the alphabet, costs some of that back in exchange for rejecting bad input.
- **It runs where others don't.** Free-threaded and Windows wheels; a reusable `no_std` Rust core for bare-metal; `#![forbid(unsafe_code)]`; streaming encode/decode; and strict decoding that refuses non-alphabet bytes instead of skipping them.

## The implementations at a glance

| Option | Strengths | Trade-offs |
| --- | --- | --- |
| **fastbase91** | Concurrency (releases the GIL on large calls); free-threaded and Windows wheels; zero-copy on `bytes`; reusable `no_std` Rust core; `#![forbid(unsafe_code)]`; streaming; strict decoding | Slower single-threaded encode than pybase91 |
| **pybase91** | Fastest single-threaded encode here, from hand-tuned scalar Rust | Holds the GIL, so no concurrency gain; the compared 0.2.2 release ships macOS/Linux wheels for Python 3.11–3.13 only — no free-threaded or Windows wheel |
| **Pure Python** | No compiled dependency; runs anywhere Python does | Roughly 100x slower than the Rust builds in these measurements |

## The other binary-to-text encodings

If you are not tied to basE91 at all, the trade-off is mostly size versus how widely supported the format is:

| Encoding | Rough size overhead | Notes |
| --- | ---: | --- |
| base64 | ~33% | Everywhere; in Python's standard library |
| base85 | ~25% | Denser than base64; in Python's standard library |
| basE91 | ~23% | Densest of the three; its 91-character alphabet uses more punctuation |

These are different formats, not different implementations of the same thing — base64 output is not basE91 output. Pick basE91 when the tighter size is worth pulling in a library; pick base64/base85 when being in the standard library and universally understood matters more.
