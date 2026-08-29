#!/usr/bin/env python3
from __future__ import annotations
import argparse, hashlib
from pathlib import Path

def main() -> int:
    p=argparse.ArgumentParser(); p.add_argument("output", type=Path); p.add_argument("files", nargs="+", type=Path); a=p.parse_args()
    lines=[]
    for path in sorted(a.files, key=lambda p:p.name):
        h=hashlib.sha256(path.read_bytes()).hexdigest(); lines.append(f"{h}  {path.name}")
    a.output.write_text("\n".join(lines)+"\n", encoding="utf-8"); return 0
if __name__ == "__main__": raise SystemExit(main())
