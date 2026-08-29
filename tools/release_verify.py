#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import sys


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify Linura release assets against SHA256SUMS")
    parser.add_argument("directory", type=Path)
    parser.add_argument("--manifest", default="SHA256SUMS")
    args = parser.parse_args()
    manifest = args.directory / args.manifest
    if not manifest.is_file():
        print(f"missing checksum manifest: {manifest}", file=sys.stderr); return 2
    failures = []
    checked = 0
    for raw in manifest.read_text(encoding="utf-8").splitlines():
        if not raw.strip(): continue
        digest, filename = raw.split(None, 1)
        filename = filename.lstrip("*")
        path = args.directory / filename
        if not path.is_file(): failures.append(f"missing asset: {filename}"); continue
        actual = sha256(path); checked += 1
        if actual != digest: failures.append(f"digest mismatch: {filename}: expected {digest}, got {actual}")
    if failures:
        for failure in failures: print(f"ERROR: {failure}", file=sys.stderr)
        return 1
    print(f"verified {checked} release assets")
    return 0


if __name__ == "__main__": raise SystemExit(main())
