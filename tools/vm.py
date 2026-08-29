#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path
import shlex
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_IMAGE = ROOT / ".artifacts/linura-dev.qcow2"


def qemu_command(image: Path, memory: int, cpus: int, ssh_port: int) -> list[str]:
    acceleration = "kvm" if Path("/dev/kvm").exists() else "tcg"
    return [
        "qemu-system-x86_64", "-machine", f"q35,accel={acceleration}", "-cpu", "host" if acceleration == "kvm" else "max",
        "-m", str(memory), "-smp", str(cpus), "-drive", f"file={image},if=virtio,format=qcow2",
        "-nic", f"user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:{ssh_port}-:22",
        "-display", "none", "-serial", "mon:stdio", "-snapshot",
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description="Linura disposable QEMU/KVM harness")
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("plan", "start"):
        p = sub.add_parser(name)
        p.add_argument("--image", type=Path, default=DEFAULT_IMAGE)
        p.add_argument("--memory", type=int, default=4096)
        p.add_argument("--cpus", type=int, default=4)
        p.add_argument("--ssh-port", type=int, default=2222)
    sub.add_parser("doctor")
    args = parser.parse_args()

    if args.command == "doctor":
        checks = {
            "qemu-system-x86_64": shutil.which("qemu-system-x86_64"),
            "ssh": shutil.which("ssh"),
            "/dev/kvm": str(Path("/dev/kvm").exists()),
        }
        for key, value in checks.items(): print(f"{key}: {value or 'missing'}")
        return 0 if checks["qemu-system-x86_64"] and checks["ssh"] else 1

    command = qemu_command(args.image, args.memory, args.cpus, args.ssh_port)
    print(shlex.join(command))
    if args.command == "plan": return 0
    if shutil.which("qemu-system-x86_64") is None:
        print("qemu-system-x86_64 is required to start a disposable VM", file=sys.stderr); return 2
    if not args.image.is_file():
        print(f"image not found: {args.image}", file=sys.stderr); return 2
    return subprocess.run(command, check=False).returncode


if __name__ == "__main__": raise SystemExit(main())
