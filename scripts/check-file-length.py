#!/usr/bin/env python3
"""Fail when a Rust source or test file exceeds the 500-line limit.

AGENTS.md keeps every *.rs file under crates/ and examples/ at 500 lines or
fewer; a module that outgrows it is split into adjacent files. Run from
anywhere in the repository; prints each oversized file with its length and
exits 1 if any.
"""

import pathlib
import sys

LIMIT = 500
ROOT = pathlib.Path(__file__).resolve().parents[1]


def main() -> int:
    oversized = []
    for top in ("crates", "examples"):
        for path in sorted((ROOT / top).rglob("*.rs")):
            if "target" in path.relative_to(ROOT).parts:
                continue
            with path.open("rb") as source:
                lines = sum(1 for _ in source)
            if lines > LIMIT:
                oversized.append((lines, path.relative_to(ROOT)))
    for lines, path in sorted(oversized, reverse=True):
        print(f"{path}: {lines} lines (limit {LIMIT})")
    return 1 if oversized else 0


if __name__ == "__main__":
    sys.exit(main())
