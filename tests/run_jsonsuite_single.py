#!/usr/bin/env python3
"""Run one JSONTestSuite file through marser's single-file test."""
# example usage:
# python3 tests/run_jsonsuite_single.py tests/JSONTestSuite/test_parsing/n_structure_100000_opening_arrays.json --mode release
from __future__ import annotations

import argparse
import os
import subprocess
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run one JSONTestSuite file.")
    parser.add_argument(
        "json_file",
        type=Path,
        help="Path to a JSONTestSuite file, e.g. tests/JSONTestSuite/test_parsing/y_*.json",
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
        default="debug",
        help="Build mode to run",
    )
    parser.add_argument(
        "--stack-bytes",
        type=int,
        default=1024 * 1024 * 1024,
        help="RUST_MIN_STACK value",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root: Path = args.repo_root.resolve()
    json_file: Path = args.json_file.resolve()
    if not json_file.exists():
        raise SystemExit(f"File does not exist: {json_file}")

    cmd = [
        "cargo",
        "test",
        "test_standard_suite_single_file_from_env",
        "--",
        "--nocapture",
    ]
    if args.mode == "release":
        cmd = [
            "cargo",
            "test",
            "--release",
            "test_standard_suite_single_file_from_env",
            "--",
            "--nocapture",
        ]

    env = os.environ.copy()
    env["JSONSUITE_FILE"] = str(json_file)
    env["RUST_MIN_STACK"] = str(args.stack_bytes)

    print(f"Running {args.mode}: {json_file.name}")
    proc = subprocess.run(cmd, cwd=repo_root, env=env)
    return proc.returncode


if __name__ == "__main__":
    raise SystemExit(main())
