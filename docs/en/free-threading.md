# Free-threading

**The short version:** if you switched to free-threaded Python to get real parallelism, one unprepared package can turn the GIL back on for the process. CPython prints a `RuntimeWarning`, but the import succeeds and the program continues with Python code in its threads serialized again, so the warning can be easy to miss and leave you with slower code. fastbase91 is built so that importing it never does that.

## The trap this avoids

Free-threaded CPython became available in 3.13 as an experimental build and has been officially supported since 3.14. Normal CPython has a global interpreter lock — the GIL — that lets only one thread run Python at a time; the no-GIL build removes it so threads genuinely run at once. fastbase91 supports free-threaded CPython 3.14t and later; 3.13t is unsupported — see [Installation & compatibility](installation.md).

Here is the catch. Compiled extensions (packages with a C or Rust part, like NumPy or this one) have to opt in to running without the GIL. If you import an extension that has **not** declared it is ready, CPython plays it safe and **turns the GIL back on for the entire process** — not just for that package — and prints a `RuntimeWarning`; the import does not fail, and the program continues. So a single dependency that has not caught up can put the Python code in your threads back to running one thread at a time.

The warning is easy to miss — for example, when output is redirected or buried in logs. Your threads keep running and your results stay correct, but their Python code runs one thread at a time again. fastbase91 one-shot calls with input of at least 1,024 bytes release the GIL and can still overlap; smaller one-shot calls and all streaming `update()` and `finish()` calls take turns — see [When a call runs in parallel](#when-a-call-runs-in-parallel). `PYTHON_GIL=1` or `-X gil=1` can also re-enable the GIL; check `sys._is_gil_enabled()` to see its current state.

## Why fastbase91 is safe to import

The Python binding is marked `#[pymodule(gil_used = false)]` — one line in the Rust source that tells CPython "this module is safe with no GIL." Because of that marker, importing fastbase91 does **not** trigger the fallback above; other extensions that have not declared support, or `PYTHON_GIL=1` / `-X gil=1`, can still re-enable the GIL.

Concretely: you build a service on free-threaded Python so eight worker threads can encode payloads in parallel. You add fastbase91 for the encoding. Import it, and `sys._is_gil_enabled()` stays `False`; your eight threads keep running at the same time. Had you reached for an extension that did not declare support and holds the GIL while it works, that one import would have re-enabled the GIL and your eight threads would be taking turns — same output, a fraction of the throughput.

A continuous-integration job proves this on every change: on a free-threaded interpreter it imports fastbase91, asserts `sys._is_gil_enabled()` is still `False`, then round-trips many payloads through concurrent encode/decode calls and checks every byte. (This proves import safety and concurrent correctness — it is not a promise that sharing one `Encoder`/`Decoder` object between threads is safe; see below.)

## The wheels we publish

Alongside the regular abi3 wheel (which covers CPython 3.11 and up), we ship dedicated free-threaded wheels for Linux x86_64, macOS arm64 (Apple silicon), and Windows x86_64; see [Installation & compatibility](installation.md) for the full platform table:

| Wheel | For |
| --- | --- |
| `cp314-cp314t` | free-threaded CPython 3.14 (the "3.14t" build) |
| `cp315` + `abi3t` ABI | free-threaded CPython 3.15t and later |

Both are on PyPI as of 0.1.1. `pip install fastbase91` picks the right one for your interpreter automatically.

## When a call runs in parallel

`fastbase91.encode()` and `fastbase91.decode()` decide per call whether to release the GIL, based on how many input bytes you gave them:

| Input size | What happens |
| --- | --- |
| 1,024 bytes or more | Release the GIL (`detach` from Python) while Rust does the work |
| under 1,024 bytes | Keep the GIL — the release/re-acquire overhead would cost more than it saves |

This applies to **both encode and decode**, and it is measured on the input you pass, not on the encoded/decoded size.

On regular (GIL-enabled) CPython, releasing the GIL on the large calls is what lets Rust work from different threads overlap, so aggregate throughput grows with the number of threads — see [Benchmarks](benchmarks.md) for measured figures. Below 1 KiB the call stays on the GIL on purpose, so do not expect that scaling for tiny inputs.

## How to actually use the threads

The unit of parallel work is one independent one-shot call, and it pays off best at 1 KiB and up:

```python
from concurrent.futures import ThreadPoolExecutor
import fastbase91

payloads = [b"x" * 64_000 for _ in range(8)]
with ThreadPoolExecutor(max_workers=4) as pool:
    encoded = list(pool.map(fastbase91.encode, payloads))
```

The streaming objects are different. `Encoder` and `Decoder` hold mutable state, so give **each thread (or each stream) its own instance** — never share one across threads. Calling `update()` on the same object from two threads at once is not a way to synchronize them; it can raise `RuntimeError`.

And if your input is backed by writable memory (a `bytearray`, say), do not let another thread modify that buffer while a call is reading it.
