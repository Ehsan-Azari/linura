#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import re
import sys

SEMANTIC_TEST = Path("crates/linura-policy/tests/approval_strength.rs")
TEST_NAME = "fn protected_approval_strength_is_actor_invariant()"
NEXT_TEST_MARKER = "\n#[test]\n"
REQUIRED_DECISION_BINDING = re.compile(
    r"if\s+let\s+PolicyDecision::RequireApproval\s*\{\s*class\s*,\s*\.\.\s*\}\s*=\s*decision"
)
REQUIRED_CLASS_COMPARISON = re.compile(
    r"assert_eq!\s*\(\s*class\s*,\s*expected_class\s*,"
)


def validate(root: Path) -> list[str]:
    path = root / SEMANTIC_TEST
    if not path.is_file():
        return [f"missing executable approval-strength contract: {SEMANTIC_TEST}"]

    text = path.read_text(encoding="utf-8")
    start = text.find(TEST_NAME)
    if start < 0:
        return [f"missing executable approval-strength test: {TEST_NAME}"]

    remainder = text[start:]
    next_test = remainder.find(NEXT_TEST_MARKER)
    test_body = remainder if next_test < 0 else remainder[:next_test]

    decision_bindings = REQUIRED_DECISION_BINDING.findall(test_body)
    if len(decision_bindings) != 1:
        return [
            "approval-strength contract must destructure exactly one runtime "
            "RequireApproval decision into `class`"
        ]

    comparisons = REQUIRED_CLASS_COMPARISON.findall(test_body)
    if len(comparisons) != 1:
        return [
            "approval-strength contract must compare the evaluated runtime `class` "
            "to the independent `expected_class` oracle exactly once"
        ]

    return []


def main(argv: list[str]) -> int:
    root = Path(argv[1]).resolve() if len(argv) > 1 else Path(__file__).resolve().parents[1]
    failures = validate(root)
    if failures:
        for failure in failures:
            print(f"ERROR: {failure}", file=sys.stderr)
        return 1
    print("approval-strength contract guard passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
