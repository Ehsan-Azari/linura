from __future__ import annotations

from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"


class ReleaseWorkflowDispatchTests(unittest.TestCase):
    def test_workflow_dispatch_commands_are_repository_explicit(self) -> None:
        failures: list[str] = []

        for workflow in sorted(WORKFLOWS.glob("*.yml")):
            lines = workflow.read_text(encoding="utf-8").splitlines()
            for index, line in enumerate(lines):
                if "gh workflow run" not in line:
                    continue

                command_lines = [line]
                cursor = index
                while command_lines[-1].rstrip().endswith("\\") and cursor + 1 < len(lines):
                    cursor += 1
                    command_lines.append(lines[cursor])

                command = "\n".join(command_lines)
                if '--repo "$GITHUB_REPOSITORY"' not in command:
                    failures.append(
                        f"{workflow.relative_to(ROOT)}:{index + 1}: "
                        "gh workflow run must pass --repo \"$GITHUB_REPOSITORY\""
                    )

        self.assertEqual([], failures, "\n".join(failures))

    def test_release_verification_dispatch_uses_release_tag_ref(self) -> None:
        release_workflow = (WORKFLOWS / "release.yml").read_text(encoding="utf-8")
        verification_dispatch = next(
            line
            for line in release_workflow.splitlines()
            if "gh workflow run release-verification.yml" in line
        )
        self.assertIn(
            '--ref "$RELEASE_TAG"',
            verification_dispatch,
            "independent verification must execute the workflow definition frozen in the published release tag",
        )


if __name__ == "__main__":
    unittest.main()
