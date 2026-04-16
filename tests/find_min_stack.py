#!/usr/bin/env python3
"""Find approximate minimum RUST_MIN_STACK for JSONTestSuite cases.

Uses binary search against the single-file Rust test:
  tests::test_standard_suite_single_file_from_env
"""

# example usage:
# python3 tests/find_min_stack.py \                                                                                           (base)
#         tests/JSONTestSuite/test_parsing/n_structure_100000_opening_arrays.json \
#         tests/JSONTestSuite/test_parsing/n_structure_open_array_object.json \
#         --mode release
from __future__ import annotations

import argparse
import math
import os
import subprocess
from pathlib import Path


def cargo_command(mode: str) -> list[str]:
    if mode == "debug":
        return [
            "cargo",
            "test",
            "test_standard_suite_single_file_from_env",
            "--",
            "--nocapture",
        ]
    if mode == "release":
        return [
            "cargo",
            "test",
            "--release",
            "test_standard_suite_single_file_from_env",
            "--",
            "--nocapture",
        ]
    raise ValueError(f"Unknown mode: {mode}")


def run_case(repo_root: Path, json_file: Path, mode: str, stack_bytes: int, timeout_s: int) -> tuple[bool, str]:
    env = os.environ.copy()
    env["JSONSUITE_FILE"] = str(json_file)
    env["RUST_MIN_STACK"] = str(stack_bytes)
    cmd = cargo_command(mode)
    try:
        proc = subprocess.run(
            cmd,
            cwd=repo_root,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=timeout_s,
        )
        out = proc.stdout
        if proc.returncode == 0:
            return True, "pass"
        lowered = out.lower()
        if "stack overflow" in lowered or "signal: 6" in lowered:
            return False, "stack_overflow"
        if "panicked at" in out:
            return False, "panic"
        return False, f"exit_{proc.returncode}"
    except subprocess.TimeoutExpired:
        return False, "timeout"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Binary-search minimum RUST_MIN_STACK.")
    parser.add_argument(
        "files",
        nargs="+",
        type=Path,
        help="One or more JSONTestSuite files to probe",
    )
    parser.add_argument(
        "--repo-root",
        default=Path(__file__).resolve().parents[1],
        type=Path,
        help="Path to marser repository root",
    )
    parser.add_argument(
        "--mode",
        choices=("debug", "release"),
        default="release",
        help="Build mode to test",
    )
    parser.add_argument(
        "--min-stack",
        type=int,
        default=64 * 1024,
        help="Lower bound for binary search in bytes",
    )
    parser.add_argument(
        "--max-stack",
        type=int,
        default=2 * 1024 * 1024 * 1024,
        help="Upper bound for binary search in bytes",
    )
    parser.add_argument(
        "--resolution-bytes",
        type=int,
        default=256 * 1024,
        help="Stop binary search when hi-lo <= this value",
    )
    parser.add_argument(
        "--timeout-seconds",
        type=int,
        default=30,
        help="Per subprocess timeout in seconds",
    )
    parser.add_argument(
        "--suggest-headroom",
        type=float,
        default=1.10,
        help="Multiplier for suggested safe stack",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = args.repo_root.resolve()
    files = [f.resolve() for f in args.files]

    for f in files:
        if not f.exists():
            raise SystemExit(f"Missing file: {f}")

    # Warm build once to avoid counting compile overhead as timeout/noise.
    warm_env = os.environ.copy()
    warm_env["JSONSUITE_FILE"] = str(files[0])
    warm_env["RUST_MIN_STACK"] = str(args.max_stack)
    subprocess.run(
        cargo_command(args.mode),
        cwd=repo_root,
        env=warm_env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    thresholds: list[int] = []
    for json_file in files:
        lo = args.min_stack
        hi = args.max_stack

        ok_hi, reason_hi = run_case(
            repo_root, json_file, args.mode, hi, args.timeout_seconds)
        if not ok_hi:
            print(f"{json_file.name}\tNO_PASS_UP_TO_MAX\t{reason_hi}")
            continue

        ok_lo, _ = run_case(repo_root, json_file, args.mode,
                            lo, args.timeout_seconds)
        if ok_lo:
            thresholds.append(lo)
            print(
                f"{json_file.name}\tMIN_APPROX_BYTES={lo}\tMIN_APPROX_KiB={math.ceil(lo / 1024)}"
            )
            continue

        while hi - lo > args.resolution_bytes:
            mid = (lo + hi) // 2
            ok_mid, _ = run_case(repo_root, json_file,
                                 args.mode, mid, args.timeout_seconds)
            if ok_mid:
                hi = mid
            else:
                lo = mid + 1

        thresholds.append(hi)
        print(
            f"{json_file.name}\tMIN_APPROX_BYTES={hi}\tMIN_APPROX_KiB={math.ceil(hi / 1024)}"
        )

    if thresholds:
        worst = max(thresholds)
        suggested = int(worst * args.suggest_headroom)
        print(f"SUGGESTED_SAFE_STACK_BYTES={suggested}")
        print(f"SUGGESTED_SAFE_STACK_KiB={math.ceil(suggested / 1024)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
