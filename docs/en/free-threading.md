# Free-threading

[正體中文](../zh/free-threading.md)

The Python extension declares `#[pymodule(gil_used = false)]`. On a free-threaded CPython build, importing fastbase91 therefore does not re-enable the GIL.

## Published wheel policies

- `cp314-cp314t` wheels target free-threaded CPython 3.14.
- `abi3t-py315` wheels use the free-threaded stable ABI for CPython 3.15t and later.

Free-threaded wheels are built for Linux, macOS, and Windows alongside regular ABI3 wheels.

## One-shot calls

`fastbase91.encode()` and `fastbase91.decode()` use an input-size threshold:

| Input size | Behavior |
| --- | --- |
| At least 1,024 bytes | Detach from Python while the Rust computation runs |
| Less than 1,024 bytes | Stay attached and avoid detach/reattach overhead |

On GIL-enabled CPython, detaching large calls lets Rust work from different Python threads overlap. The repository benchmark records about 1,675 MiB/s aggregate encode throughput at four threads. Small calls deliberately stay attached because the detach cost matters more at that size; do not expect the same GIL-release scaling below 1 KiB.

This threshold applies to the number of input bytes passed to each one-shot call, not the eventual encoded or decoded size.

## Recommended threading model

Independent one-shot calls are the intended unit of parallel work, especially for inputs of at least 1 KiB:

```python
from concurrent.futures import ThreadPoolExecutor
import fastbase91

payloads = [b"x" * 64_000 for _ in range(8)]
with ThreadPoolExecutor(max_workers=4) as pool:
    encoded = list(pool.map(fastbase91.encode, payloads))
```

`Encoder` and `Decoder` are mutable streaming state machines. Create a separate instance for every stream or thread; do not share one instance between concurrent threads. A concurrent `update()` on the same Python instance is not a synchronization mechanism and can raise `RuntimeError`.

If input is backed by writable memory, do not mutate that buffer concurrently while a call is reading it.

## Continuous-integration gate

The free-threaded wheel job runs on a free-threaded interpreter and verifies both of these properties:

1. importing `fastbase91` leaves `sys._is_gil_enabled()` false;
2. many payloads still round-trip correctly through concurrent encode/decode calls.

These checks cover import behavior and concurrent correctness. They are not a claim that sharing mutable streaming objects is thread-safe.

