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
RUST_CHAR_LITERAL = re.compile(
    r"'(?:[^'\\\n]|\\(?:[nrt0\\'\"]|x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]{1,6}\}))'"
)


def strip_rust_comments_and_literals(text: str) -> str:
    """Replace non-code Rust comments/literals while preserving layout."""
    output: list[str] = []
    index = 0
    block_depth = 0

    while index < len(text):
        if block_depth:
            if text.startswith("/*", index):
                block_depth += 1
                output.extend("  ")
                index += 2
            elif text.startswith("*/", index):
                block_depth -= 1
                output.extend("  ")
                index += 2
            else:
                character = text[index]
                output.append("\n" if character == "\n" else " ")
                index += 1
            continue

        if text.startswith("//", index):
            while index < len(text) and text[index] != "\n":
                output.append(" ")
                index += 1
            continue

        if text.startswith("/*", index):
            block_depth = 1
            output.extend("  ")
            index += 2
            continue

        # Strip valid Rust character literals before string handling. This is
        # deliberately anchored to a complete char literal so lifetimes such as
        # `'a` remain ordinary Rust tokens rather than swallowing later source.
        if text[index] == "'":
            char_literal = RUST_CHAR_LITERAL.match(text, index)
            if char_literal is not None:
                stop = char_literal.end()
                output.extend(" " * (stop - index))
                index = stop
                continue

        if text[index] == "r":
            delimiter = index + 1
            while delimiter < len(text) and text[delimiter] == "#":
                delimiter += 1
            if delimiter < len(text) and text[delimiter] == '"':
                hashes = delimiter - (index + 1)
                terminator = '"' + ("#" * hashes)
                end = text.find(terminator, delimiter + 1)
                stop = len(text) if end < 0 else end + len(terminator)
                while index < stop:
                    character = text[index]
                    output.append("\n" if character == "\n" else " ")
                    index += 1
                continue

        if text[index] == '"':
            output.append(" ")
            index += 1
            escaped = False
            while index < len(text):
                character = text[index]
                output.append("\n" if character == "\n" else " ")
                index += 1
                if escaped:
                    escaped = False
                elif character == "\\":
                    escaped = True
                elif character == '"':
                    break
            continue

        output.append(text[index])
        index += 1

    return "".join(output)


def validate(root: Path) -> list[str]:
    path = root / SEMANTIC_TEST
    if not path.is_file():
        return [f"missing executable approval-strength contract: {SEMANTIC_TEST}"]

    code = strip_rust_comments_and_literals(path.read_text(encoding="utf-8"))
    start = code.find(TEST_NAME)
    if start < 0:
        return [f"missing executable approval-strength test: {TEST_NAME}"]

    remainder = code[start:]
    next_test = remainder.find(NEXT_TEST_MARKER)
    test_body = remainder if next_test < 0 else remainder[:next_test]

    decision_bindings = REQUIRED_DECISION_BINDING.findall(test_body)
    if len(decision_bindings) != 1:
        return [
            "approval-strength contract must destructure exactly one executable runtime "
            "RequireApproval decision into `class`"
        ]

    comparisons = REQUIRED_CLASS_COMPARISON.findall(test_body)
    if len(comparisons) != 1:
        return [
            "approval-strength contract must compare the executable evaluated runtime `class` "
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
