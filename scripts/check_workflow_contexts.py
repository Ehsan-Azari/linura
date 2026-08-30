#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW_DIR = ROOT / ".github" / "workflows"
ENV_HEADER = re.compile(r"^(?P<indent>\s*)env:\s*(?P<value>.*)$")
RUNNER_EXPRESSION = re.compile(r"\$\{\{\s*runner\.")


def leading_spaces(line: str) -> int:
    return len(line) - len(line.lstrip(" "))


def main() -> int:
    failures: list[str] = []

    for workflow in sorted((*WORKFLOW_DIR.glob("*.yml"), *WORKFLOW_DIR.glob("*.yaml"))):
        lines = workflow.read_text(encoding="utf-8").splitlines()
        for index, line in enumerate(lines):
            match = ENV_HEADER.match(line)
            if match is None:
                continue

            env_indent = len(match.group("indent"))
            # Repository policy is deliberately stricter than GitHub's context
            # matrix: runner.* expressions are forbidden in every env mapping.
            # This avoids scope/indentation heuristics while preserving runner.*
            # for valid non-env step contexts such as `with:` when necessary.
            if RUNNER_EXPRESSION.search(match.group("value")):
                failures.append(
                    f"runner context is forbidden in env mappings: "
                    f"{workflow.relative_to(ROOT)}:{index + 1}"
                )

            # Cover ordinary block mappings and multi-line flow mappings using
            # indentation relative to this specific env key, not fixed columns.
            for nested_index in range(index + 1, len(lines)):
                nested = lines[nested_index]
                if not nested.strip():
                    continue
                if leading_spaces(nested) <= env_indent:
                    break
                if RUNNER_EXPRESSION.search(nested):
                    failures.append(
                        f"runner context is forbidden in env mappings: "
                        f"{workflow.relative_to(ROOT)}:{nested_index + 1}"
                    )

    if failures:
        for failure in failures:
            print(f"ERROR: {failure}", file=sys.stderr)
        return 1

    print("GitHub workflow context checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
