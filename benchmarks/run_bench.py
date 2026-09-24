"""Cross-implementation basE91 throughput + interop benchmark.

Contenders: fastbase91 (Rust/PyO3, ours), pybase91 (Rust/PyO3, third party),
and the pure-Python baseline copied from the user's project.
"""
import os
import random
import sys
import time

sys.path.insert(0, os.path.dirname(__file__))
import purepy_base91 as pp  # base91_encode(bytes)->str, base91_decode(str)->bytes
import fastbase91 as fb
import pybase91 as pb

print("== versions / API ==")
print("fastbase91:", fb.__version__, "core", fb.CORE_VERSION)
print("pybase91:", getattr(pb, "__version__", "?"))
print("pybase91 public names:", [n for n in dir(pb) if not n.startswith("_")])

pb_encode = getattr(pb, "encode", None) or getattr(pb, "b91encode", None) or getattr(pb, "encode_bytes", None)
pb_decode = getattr(pb, "decode", None) or getattr(pb, "b91decode", None) or getattr(pb, "decode_bytes", None)
print("pybase91 encode ->", pb_encode, "| decode ->", pb_decode)

# encode: bytes -> (str|bytes); decode: (str|bytes) -> bytes
impls = {
    "fastbase91 (Rust)": (lambda b: fb.encode(b), lambda e: fb.decode(e)),
    "pure-Python": (lambda b: pp.base91_encode(b), lambda e: pp.base91_decode(e)),
    "pybase91 (Rust)": (lambda b: pb_encode(b), lambda e: pb_decode(e)),
}

def as_text(e):
    return e.decode("ascii") if isinstance(e, (bytes, bytearray)) else e

print("\n== correctness (roundtrip on 0..255) ==")
sample = bytes(range(256))
enc_out = {}
for name, (enc, dec) in impls.items():
    e = enc(sample)
    enc_out[name] = e
    ok = dec(e) == sample
    print(f"{name:20} enc_type={type(e).__name__:5} enc_len={len(e):4} roundtrip={ok}")

print("\n== interop (are the encoded forms identical text?) ==")
texts = {k: as_text(v) for k, v in enc_out.items()}
base = texts["fastbase91 (Rust)"]
for name, t in texts.items():
    print(f"{name:20} == fastbase91 output: {t == base}")

def bench(fn, arg, budget=0.6):
    # warm up once, then loop for a fixed time budget and report best-effort throughput
    fn(arg)
    n = 0
    t0 = time.perf_counter()
    while True:
        fn(arg)
        n += 1
        dt = time.perf_counter() - t0
        if dt >= budget:
            break
    return n, dt

print("\n== throughput (random bytes; MiB/s of INPUT bytes) ==")
MIB = 1024 * 1024
for size in [1024, 64 * 1024, 1024 * 1024]:
    data = bytes(random.getrandbits(8) for _ in range(size))
    print(f"\n--- input size {size} bytes ---")
    print(f"{'impl':20} {'encode MiB/s':>14} {'decode MiB/s':>14}")
    for name, (enc, dec) in impls.items():
        encoded = enc(data)
        n, dt = bench(enc, data)
        enc_mibs = (n * size) / MIB / dt
        m, dt2 = bench(dec, encoded)
        dec_mibs = (m * size) / MIB / dt2
        print(f"{name:20} {enc_mibs:14.1f} {dec_mibs:14.1f}")

from concurrent.futures import ThreadPoolExecutor
import sys as _sys

def concurrency_bench(enc, arg, n_threads, budget=0.6):
    stop = time.perf_counter() + budget
    counts = [0] * n_threads
    def work(i):
        c = 0
        while time.perf_counter() < stop:
            enc(arg)
            c += 1
        counts[i] = c
    t0 = time.perf_counter()
    with ThreadPoolExecutor(max_workers=n_threads) as ex:
        list(ex.map(work, range(n_threads)))
    dt = time.perf_counter() - t0
    return sum(counts) * len(arg) / MIB / dt

gil = getattr(_sys, "_is_gil_enabled", lambda: True)()
print(f"\n== concurrency scaling (encode 64 KiB; aggregate MiB/s; GIL enabled={gil}) ==")
print("fastbase91 releases the GIL on >=1 KiB inputs, so its Rust compute can overlap across threads.")
cdata = bytes(random.getrandbits(8) for _ in range(64 * 1024))
print(f"{'impl':20} {'1 thread':>10} {'2 threads':>10} {'4 threads':>10}")
for name, (enc, dec) in impls.items():
    cells = []
    for nt in (1, 2, 4):
        cells.append(f"{concurrency_bench(enc, cdata, nt):10.0f}")
    print(f"{name:20} " + " ".join(cells))
