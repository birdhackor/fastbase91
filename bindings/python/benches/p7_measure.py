#!/usr/bin/env python3
"""P7 full-path measurement harness for the installed Python extension."""

from __future__ import annotations

import argparse
import json
import os
import platform
import resource
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from statistics import median

import fastbase91


MIB = 1024 * 1024
FULL_SIZES = (1 << 10, 1 << 16, 1 << 20)
SMALL_SIZES = (0, 8, 64, 256, 1 << 10, 4 << 10, 16 << 10, 64 << 10)
THREAD_COUNTS = (1, 2, 4, 8)
PATTERNS = ("random", "zero", "ff", "text")
LCG_SEED = 0x4D59_5DF4_D0F3_3173
LCG_MULTIPLIER = 6_364_136_223_846_793_005
LCG_INCREMENT = 1_442_695_040_888_963_407
U64_MASK = (1 << 64) - 1
TEXT = (
    "The quick brown fox jumps over the lazy dog. "
    "basE91 is a compact binary-to-text encoding. "
    "繁體中文的實際文字資料也在這個固定樣本中。\n"
).encode("utf-8")


def random_bytes(length, seed=LCG_SEED):
    """Match the deterministic LCG in crates/fastbase91-core/benches/throughput.rs."""
    state = seed
    data = bytearray(length)
    for index in range(length):
        state = (state * LCG_MULTIPLIER + LCG_INCREMENT) & U64_MASK
        data[index] = (state >> 32) & 0xFF
    return bytes(data)


def payload(pattern, length):
    if pattern == "random":
        return random_bytes(length)
    if pattern == "zero":
        return bytes(length)
    if pattern == "ff":
        return b"\xff" * length
    if pattern == "text":
        return (TEXT * ((length + len(TEXT) - 1) // len(TEXT)))[:length]
    raise ValueError(f"unknown pattern: {pattern}")


def median_call_ns(operation, expected, warmups, samples):
    """Time exactly one public call per sample; validate result after stopping timer."""
    for _ in range(warmups):
        if operation() != expected:
            raise AssertionError("warmup result differs from expected bytes")
    elapsed_ns = []
    for _ in range(samples):
        started = time.perf_counter_ns()
        result = operation()
        elapsed_ns.append(time.perf_counter_ns() - started)
        if result != expected:
            raise AssertionError("measured result differs from expected bytes")
    return elapsed_ns, int(median(elapsed_ns))


def record_call(operation_name, source, expected, warmups, samples, pattern_name=None):
    function = fastbase91.encode if operation_name == "encode" else fastbase91.decode
    elapsed_ns, median_ns = median_call_ns(
        lambda: function(source), expected, warmups, samples
    )
    record = {
        "operation": operation_name,
        "source_bytes": len(source),
        "output_bytes": len(expected),
        "denominator_bytes": len(source) if operation_name == "encode" else len(expected),
        "denominator_description": (
            "input bytes" if operation_name == "encode" else "decoded/output bytes"
        ),
        "warmup_calls": warmups,
        "measured_calls": samples,
        "elapsed_ns_samples": elapsed_ns,
        "median_ns": median_ns,
    }
    if pattern_name is not None:
        record["pattern"] = pattern_name
    return record


def add_throughput(record):
    record["throughput_mib_s"] = record["denominator_bytes"] / MIB / (
        record["median_ns"] / 1_000_000_000
    )


def full_throughput(warmups, samples):
    records = []
    for size in FULL_SIZES:
        for pattern_name in PATTERNS:
            original = payload(pattern_name, size)
            encoded = fastbase91.encode(original)
            if fastbase91.decode(encoded) != original:
                raise AssertionError("setup round trip failed")
            for operation, source, expected in (
                ("encode", original, encoded),
                ("decode", encoded, original),
            ):
                record = record_call(
                    operation, source, expected, warmups, samples, pattern_name
                )
                record["original_size_bytes"] = size
                add_throughput(record)
                records.append(record)
    return records


def small_latency(warmups, samples):
    records = []
    for size in SMALL_SIZES:
        original = payload("random", size)
        encoded = fastbase91.encode(original)
        for operation, source, expected in (
            ("encode", original, encoded),
            ("decode", encoded, original),
        ):
            record = record_call(operation, source, expected, warmups, samples, "random")
            record["original_size_bytes"] = size
            records.append(record)
    return records


def rss_child(kind, size):
    if kind == "bytes":
        source = bytes(size)
    elif kind == "bytearray":
        # No temporary bytes object: its allocation would contaminate peak RSS.
        source = bytearray(size)
    else:
        raise ValueError(f"unknown RSS input kind: {kind}")
    output = fastbase91.encode(source)
    if not output:
        raise AssertionError("non-empty RSS source unexpectedly encoded to empty output")
    raw_peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    unit = "bytes" if sys.platform == "darwin" else "KiB"
    print(
        json.dumps(
            {
                "input_kind": kind,
                "input_size_bytes": size,
                "output_size_bytes": len(output),
                "ru_maxrss_raw": raw_peak,
                "ru_maxrss_unit": unit,
                "ru_maxrss_bytes": raw_peak if unit == "bytes" else raw_peak * 1024,
            },
            sort_keys=True,
        )
    )


def peak_rss(size):
    results = []
    script = Path(__file__).resolve()
    for kind in ("bytes", "bytearray"):
        completed = subprocess.run(
            [sys.executable, str(script), "rss-child", "--kind", kind, "--size", str(size)],
            check=True,
            capture_output=True,
            text=True,
        )
        results.append(json.loads(completed.stdout))
    by_kind = {item["input_kind"]: item for item in results}
    return {
        "protocol": "separate subprocess per input kind; one one-shot encode per child",
        "results": results,
        "bytearray_minus_bytes_peak_bytes": (
            by_kind["bytearray"]["ru_maxrss_bytes"]
            - by_kind["bytes"]["ru_maxrss_bytes"]
        ),
    }


def scaling_operation(operation, size, threads, warmups, samples):
    originals = [random_bytes(size, LCG_SEED ^ (index + 1)) for index in range(threads)]
    encoded = [fastbase91.encode(item) for item in originals]
    if operation == "encode":
        sources, expected, function = originals, encoded, fastbase91.encode
    else:
        sources, expected, function = encoded, originals, fastbase91.decode

    def run_once(pool):
        started = time.perf_counter_ns()
        result = list(pool.map(function, sources))
        elapsed = time.perf_counter_ns() - started
        if result != expected:
            raise AssertionError(f"parallel {operation} result differs from one-shot result")
        return elapsed

    with ThreadPoolExecutor(max_workers=threads) as pool:
        for _ in range(warmups):
            run_once(pool)
        elapsed_ns = [run_once(pool) for _ in range(samples)]
    median_ns = int(median(elapsed_ns))
    denominator_bytes = threads * size
    return {
        "operation": operation,
        "threads": threads,
        "payloads_are_independent": True,
        "original_size_bytes_per_payload": size,
        "denominator_bytes": denominator_bytes,
        "denominator_description": (
            "total input bytes" if operation == "encode" else "total decoded/output bytes"
        ),
        "warmup_batches": warmups,
        "measured_batches": samples,
        "elapsed_ns_samples": elapsed_ns,
        "median_ns": median_ns,
        "total_throughput_mib_s": denominator_bytes / MIB / (median_ns / 1_000_000_000),
        "correctness_assertion": "passed: every batch byte-equals its single-thread one-shot result",
    }


def scaling(warmups, samples):
    return [
        scaling_operation(operation, 1 << 20, threads, warmups, samples)
        for operation in ("encode", "decode")
        for threads in THREAD_COUNTS
    ]


def environment(variant):
    return {
        "variant_label": variant,
        "build_profile": "release",
        "min_detach_len": fastbase91._fastbase91._MIN_DETACH_LEN,
        "timer": "time.perf_counter_ns",
        "python": sys.version,
        "executable": sys.executable,
        "platform": platform.platform(),
        "machine": platform.machine(),
        "processor": platform.processor(),
        "pid": os.getpid(),
    }


def run_measurement(args):
    result = {
        "environment": environment(args.variant),
        "methodology": {
            "full_path_scope": (
                "existing input borrowing or copying, Rust compute, Rust output Vec allocation and "
                "zeroing, Vec-to-Python-bytes copy, and Python bytes object construction are timed"
            ),
            "input_type_for_timing": "bytes (borrowed binding path)",
            "full_throughput": {
                "sizes_bytes": list(FULL_SIZES),
                "patterns": list(PATTERNS),
                "warmup_calls": args.full_warmups,
                "measured_calls": args.full_samples,
            },
            "small_latency": {
                "sizes_bytes": list(SMALL_SIZES),
                "pattern": "random deterministic LCG matching core throughput.rs",
                "warmup_calls": args.small_warmups,
                "measured_calls": args.small_samples,
            },
            "scaling": {
                "payload_size_bytes": 1 << 20,
                "threads": list(THREAD_COUNTS),
                "warmup_batches": args.scaling_warmups,
                "measured_batches": args.scaling_samples,
            },
        },
        "full_throughput": full_throughput(args.full_warmups, args.full_samples),
        "small_latency": small_latency(args.small_warmups, args.small_samples),
    }
    if not args.skip_rss:
        result["peak_rss"] = peak_rss(args.rss_size)
    if not args.skip_scaling:
        result["scaling"] = scaling(args.scaling_warmups, args.scaling_samples)
    return result


def write_json(result, output):
    rendered = json.dumps(result, indent=2, sort_keys=True)
    if output is None:
        print(rendered)
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered + "\n", encoding="utf-8")
        print(f"wrote raw JSON: {output}")


def print_records(title, records, columns):
    print(title)
    print("\t".join(columns))
    for record in records:
        values = []
        for column in columns:
            value = record[column]
            values.append(f"{value:.2f}" if isinstance(value, float) else str(value))
        print("\t".join(values))


def render_result(result):
    env = result["environment"]
    print(
        f"variant={env['variant_label']} build_profile={env['build_profile']} "
        f"timer={env['timer']} platform={env['platform']}"
    )
    print_records(
        "FULL_PATH",
        result["full_throughput"],
        [
            "operation", "original_size_bytes", "pattern", "denominator_description",
            "median_ns", "throughput_mib_s",
        ],
    )
    print_records(
        "SMALL_LATENCY",
        result["small_latency"],
        ["operation", "original_size_bytes", "median_ns", "measured_calls"],
    )
    if "peak_rss" in result:
        print("PEAK_RSS")
        for item in result["peak_rss"]["results"]:
            print(
                f"input_kind={item['input_kind']} input_size_bytes={item['input_size_bytes']} "
                f"ru_maxrss={item['ru_maxrss_raw']} {item['ru_maxrss_unit']} "
                f"ru_maxrss_bytes={item['ru_maxrss_bytes']}"
            )
        print(
            "bytearray_minus_bytes_peak_bytes="
            f"{result['peak_rss']['bytearray_minus_bytes_peak_bytes']}"
        )
    if "scaling" in result:
        print_records(
            "SCALING",
            result["scaling"],
            ["operation", "threads", "median_ns", "total_throughput_mib_s", "correctness_assertion"],
        )


def index_record(records, operation, size):
    for record in records:
        if record["operation"] == operation and record["original_size_bytes"] == size:
            return record
    raise KeyError(f"missing {operation} record at {size} bytes")


def parse_core_output(output):
    records = []
    for line in output.splitlines():
        if line.startswith("case="):
            fields = dict(item.split("=", 1) for item in line.split() if "=" in item)
            records.append(
                {
                    "case": fields["case"],
                    "size_bytes": int(fields["size"].removesuffix("B")),
                    "input_bytes": int(fields["input_bytes"]),
                    "elapsed_ns": int(fields["elapsed_ns"]),
                    "throughput_mib_s": float(fields["throughput_mib_s"]),
                }
            )
    if not records:
        raise ValueError("no throughput records found in cargo bench output")
    return records


def core_bench():
    repo = Path(__file__).resolve().parents[3]
    command = ["cargo", "bench", "-p", "fastbase91-core"]
    completed = subprocess.run(command, cwd=repo, capture_output=True, text=True)
    print(f"command={' '.join(command)}")
    print(f"returncode={completed.returncode}")
    if completed.stdout:
        print("stdout:")
        print(completed.stdout, end="" if completed.stdout.endswith("\n") else "\n")
    if completed.stderr:
        print("stderr:", file=sys.stderr)
        print(completed.stderr, end="" if completed.stderr.endswith("\n") else "\n", file=sys.stderr)
    if completed.returncode:
        raise SystemExit(completed.returncode)
    return {
        "command": command,
        "returncode": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
        "records": parse_core_output(completed.stdout),
    }


def core_record(records, case, size):
    for record in records:
        if record["case"] == case and record["size_bytes"] == size:
            return record
    raise KeyError(f"missing core {case} record at {size} bytes")


def compare(args):
    shipping = json.loads(args.shipping.read_text(encoding="utf-8"))
    no_detach = json.loads(args.no_detach.read_text(encoding="utf-8"))
    core = json.loads(args.core.read_text(encoding="utf-8"))
    ship_floor = shipping["environment"]["min_detach_len"]
    no_floor = no_detach["environment"]["min_detach_len"]
    print("DETACH_COST_VS_CORE")
    print(
        f"# shipping detaches at input >= {ship_floor} bytes; "
        f"no_detach detaches at input >= {no_floor} bytes; "
        "detach_delta_ns is NA unless exactly the shipping side detaches"
    )
    print(
        "operation\toriginal_size_bytes\tsource_bytes\tshipping_ns\tno_detach_ns\t"
        "detach_delta_ns\tcore_one_shot_ns\tcore_slice_preallocated_ns"
    )
    for operation in ("encode", "decode"):
        for size in SMALL_SIZES:
            ship = index_record(shipping["small_latency"], operation, size)
            no = index_record(no_detach["small_latency"], operation, size)
            source_bytes = ship["source_bytes"]
            ship_detaches = source_bytes >= ship_floor
            no_detaches = no["source_bytes"] >= no_floor
            if ship_detaches and not no_detaches:
                delta = str(ship["median_ns"] - no["median_ns"])
            else:
                delta = (
                    f"NA (shipping_detaches={ship_detaches} "
                    f"no_detach_detaches={no_detaches}; not a detach vs no-detach pair)"
                )
            prefix = (
                f"{operation}\t{size}\t{source_bytes}\t{ship['median_ns']}\t"
                f"{no['median_ns']}\t{delta}"
            )
            if size in (1 << 10, 64 << 10):
                core_one = core_record(core["records"], f"{operation}-one-shot", size)
                core_slice = core_record(core["records"], f"{operation}-slice", size)
                print(f"{prefix}\t{core_one['elapsed_ns']}\t{core_slice['elapsed_ns']}")
            else:
                print(f"{prefix}\tNA (core bench size not emitted)\tNA")
    print("FULL_PATH_COMPOSITION_RANDOM_1MIB")
    print(
        "operation\tpython_full_ns\tcore_slice_preallocated_ns\tcore_one_shot_ns\t"
        "core_one_shot_pct_of_python\tpython_boundary_plus_pybytes_ns"
    )
    for operation in ("encode", "decode"):
        full = next(
            item for item in shipping["full_throughput"]
            if item["operation"] == operation and item["pattern"] == "random"
            and item["original_size_bytes"] == 1 << 20
        )
        one = core_record(core["records"], f"{operation}-one-shot", 1 << 20)
        slice_record = core_record(core["records"], f"{operation}-slice", 1 << 20)
        print(
            f"{operation}\t{full['median_ns']}\t{slice_record['elapsed_ns']}\t"
            f"{one['elapsed_ns']}\t{one['elapsed_ns'] / full['median_ns'] * 100:.2f}\t"
            f"{full['median_ns'] - one['elapsed_ns']}"
        )
    print(
        "NOTE: core slice preallocates output; core one-shot includes its Vec allocation. "
        "The residual is a timing decomposition, not a profiler attribution."
    )


def parser():
    root = argparse.ArgumentParser(description=__doc__)
    subparsers = root.add_subparsers(dest="command", required=True)
    run = subparsers.add_parser("run", help="measure installed extension")
    run.add_argument("--variant", required=True, choices=("shipping", "bench_no_detach"))
    run.add_argument("--output", type=Path)
    run.add_argument("--full-warmups", type=int, default=3)
    run.add_argument("--full-samples", type=int, default=13)
    run.add_argument("--small-warmups", type=int, default=9)
    run.add_argument("--small-samples", type=int, default=31)
    run.add_argument("--scaling-warmups", type=int, default=3)
    run.add_argument("--scaling-samples", type=int, default=11)
    run.add_argument("--rss-size", type=int, default=64 << 20)
    run.add_argument("--skip-rss", action="store_true")
    run.add_argument("--skip-scaling", action="store_true")
    child = subparsers.add_parser("rss-child", help=argparse.SUPPRESS)
    child.add_argument("--kind", required=True, choices=("bytes", "bytearray"))
    child.add_argument("--size", required=True, type=int)
    core = subparsers.add_parser("core", help="run existing core throughput bench")
    core.add_argument("--output", type=Path)
    render = subparsers.add_parser("render", help="render compact raw-result tables")
    render.add_argument("--input", required=True, type=Path)
    comparison = subparsers.add_parser("compare", help="join variants with core results")
    comparison.add_argument("--shipping", required=True, type=Path)
    comparison.add_argument("--no-detach", required=True, type=Path)
    comparison.add_argument("--core", required=True, type=Path)
    return root


def main():
    args = parser().parse_args()
    if args.command == "rss-child":
        rss_child(args.kind, args.size)
    elif args.command == "run":
        write_json(run_measurement(args), args.output)
    elif args.command == "core":
        write_json(core_bench(), args.output)
    elif args.command == "render":
        render_result(json.loads(args.input.read_text(encoding="utf-8")))
    elif args.command == "compare":
        compare(args)


if __name__ == "__main__":
    main()
