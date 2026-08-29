#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
import shlex
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SCENARIOS = ROOT / "tests/acceptance"


def scenarios() -> list[dict]:
    result = []
    for path in sorted(SCENARIOS.glob("*.json")):
        item = json.loads(path.read_text(encoding="utf-8"))
        item["_path"] = path
        result.append(item)
    return result


def find_scenario(scenario_id: str) -> dict:
    for scenario in scenarios():
        if scenario["id"] == scenario_id:
            return scenario
    raise SystemExit(f"unknown acceptance scenario: {scenario_id}")


def ssh_base(host: str, user: str, port: int, identity: str | None) -> list[str]:
    command = [
        "ssh", "-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=no",
        "-o", "UserKnownHostsFile=/dev/null", "-o", "ConnectTimeout=10", "-p", str(port),
    ]
    if identity:
        command.extend(["-i", identity])
    command.append(f"{user}@{host}")
    return command


def main() -> int:
    parser = argparse.ArgumentParser(description="Linura disposable-machine acceptance runner")
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("list")
    run = sub.add_parser("run")
    run.add_argument("scenario")
    run.add_argument("--host", default="127.0.0.1")
    run.add_argument("--user", default="linura")
    run.add_argument("--port", type=int, default=2222)
    run.add_argument("--identity")
    run.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    if args.command == "list":
        for item in scenarios():
            print(f"{item['id']}: {item['description']}")
        return 0

    scenario = find_scenario(args.scenario)
    print(f"scenario: {scenario['id']} — {scenario['description']}")
    for step in scenario["steps"]:
        command = ssh_base(args.host, args.user, args.port, args.identity) + [step["command"]]
        print(f"[{step['name']}] {shlex.join(command)}")
        if not args.dry_run:
            completed = subprocess.run(command, check=False)
            if completed.returncode != 0:
                print(f"FAILED: {step['name']} exited {completed.returncode}", file=sys.stderr)
                return completed.returncode
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
