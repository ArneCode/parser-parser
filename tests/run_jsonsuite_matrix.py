#!/usr/bin/env python3
"""Run JSONTestSuite per-file matrix in debug/release.

This script runs each file in tests/JSONTestSuite/test_parsing in an isolated
cargo test subprocess using the env-driven Rust test:
  tests::test_standard_suite_single_file_from_env
"""
# example usage:
# python3 tests/run_jsonsuite_matrix.py --mode both
from __future__ import annotations

import argparse
import csv
import os
import subprocess
from pathlib import Path


def classify_status(exit_code: int, output: str) -> str:
    lowered = output.lower()
    if exit_code == 0:
        return "pass"
    if "stack overflow" in lowered or "signal: 6" in lowered:
        return "crash_stack_overflow"
    if "panicked at" in lowered:
        return "panic"
    return "fail"


def mode_command(mode: str) -> list[str]:
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


def run_mode(
    repo_root: Path,
    files: list[Path],
    mode: str,
    stack_bytes: int,
) -> list[tuple[str, str, str, int]]:
    rows: list[tuple[str, str, str, int]] = []
    cmd = mode_command(mode)

    for json_file in files:
        env = os.environ.copy()
        env["JSONSUITE_FILE"] = str(json_file)
        env["RUST_MIN_STACK"] = str(stack_bytes)

        proc = subprocess.run(
            cmd,
            cwd=repo_root,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        status = classify_status(proc.returncode, proc.stdout)
        rows.append((mode, json_file.name, status, proc.returncode))

    return rows


def write_outputs(
    rows: list[tuple[str, str, str, int]],
    csv_path: Path,
    summary_path: Path,
) -> None:
    csv_path.parent.mkdir(parents=True, exist_ok=True)
    summary_path.parent.mkdir(parents=True, exist_ok=True)

    with csv_path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(["mode", "file", "status", "exit_code"])
        writer.writerows(rows)

    by_mode: dict[str, dict[str, object]] = {}
    for mode, file_name, status, exit_code in rows:
        if mode not in by_mode:
            by_mode[mode] = {
                "pass": 0,
                "panic": 0,
                "crash_stack_overflow": 0,
                "fail": 0,
                "files": [],
            }
        by_mode[mode][status] = int(by_mode[mode][status]) + 1
        if status != "pass":
            by_mode[mode]["files"].append((file_name, status, exit_code))

    with summary_path.open("w", encoding="utf-8") as f:
        for mode in ("debug", "release"):
            if mode not in by_mode:
                continue
            s = by_mode[mode]
            f.write(
                f"[{mode}] pass={s['pass']} fail={s['fail']} "
                f"panic={s['panic']} crash_stack_overflow={s['crash_stack_overflow']}\n"
            )
            for file_name, status, exit_code in s["files"]:
                f.write(f"  - {file_name}: {status} (exit={exit_code})\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run JSONTestSuite per-file matrix.")
    parser.add_argument(
        "--repo-root",
        default=Path(__file__).resolve().parents[1],
        type=Path,
        help="Path to marser repository root",
    )
    parser.add_argument(
        "--suite-dir",
        default=None,
        type=Path,
        help="Directory containing JSON test files (defaults to tests/JSONTestSuite/test_parsing)",
    )
    parser.add_argument(
        "--mode",
        choices=("debug", "release", "both"),
        default="both",
        help="Which build mode(s) to run",
    )
    parser.add_argument(
        "--stack-bytes",
        type=int,
        default=1024 * 1024 * 1024,
        help="RUST_MIN_STACK value to use for subprocesses",
    )
    parser.add_argument(
        "--csv-out",
        default=None,
        type=Path,
        help="Output CSV path (defaults to tests/jsonsuite-per-file-matrix.csv)",
    )
    parser.add_argument(
        "--summary-out",
        default=None,
        type=Path,
        help="Output summary path (defaults to tests/jsonsuite-per-file-summary.txt)",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root: Path = args.repo_root.resolve()
    suite_dir: Path = (
        args.suite_dir.resolve()
        if args.suite_dir is not None
        else repo_root / "tests/JSONTestSuite/test_parsing"
    )
    csv_out = (
        args.csv_out.resolve()
        if args.csv_out is not None
        else repo_root / "tests/jsonsuite-per-file-matrix.csv"
    )
    summary_out = (
        args.summary_out.resolve()
        if args.summary_out is not None
        else repo_root / "tests/jsonsuite-per-file-summary.txt"
    )

    files = sorted(suite_dir.glob("*.json"))
    if not files:
        raise SystemExit(f"No JSON files found in {suite_dir}")

    modes = ("debug", "release") if args.mode == "both" else (args.mode,)
    rows: list[tuple[str, str, str, int]] = []
    for mode in modes:
        print(f"Running {mode} on {len(files)} files...")
        rows.extend(run_mode(repo_root, files, mode, args.stack_bytes))

    write_outputs(rows, csv_out, summary_out)
    print(f"Wrote {csv_out}")
    print(f"Wrote {summary_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
