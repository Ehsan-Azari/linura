from __future__ import annotations

from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class AuthorityFoundationTests(unittest.TestCase):
    def _run_checker(self, root: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(ROOT / "tools/check_authority_foundation.py"), str(root)],
            capture_output=True,
            text=True,
            check=False,
        )

    def _copy_fixture(self, destination: Path) -> None:
        paths = (
            "contracts/roadmap.toml",
            "crates/linura-core/src/lib.rs",
            "crates/linura-protocol/src/lib.rs",
            "crates/linura-provider-sdk/src/lib.rs",
            "crates/linura-control/src/lib.rs",
            "crates/linura-lifecycle/src/lib.rs",
            "crates/linura-planner/src/lib.rs",
            "executors/linura-executor-systemd/src/lib.rs",
            "docs/milestones/v0.3.0.md",
            "docs/qualification/v0.3.0.md",
            "docs/adr/0018-canonical-plan-review-authority.md",
            "docs/threat-model.md",
        )
        for relative in paths:
            source = ROOT / relative
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)

    def test_repository_authority_foundation_is_valid(self) -> None:
        result = self._run_checker(ROOT)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_v03_cannot_promote_review_to_execution_authority(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            contract = root / "contracts/roadmap.toml"
            text = contract.read_text(encoding="utf-8").replace(
                'authority_state = "review-only"',
                'authority_state = "lifecycle-integrated"',
                1,
            )
            contract.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("v0.3.0: authority_state must remain 'review-only'", result.stderr)

    def test_removed_action_plan_cannot_be_reintroduced(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            core = root / "crates/linura-core/src/lib.rs"
            core.write_text(
                core.read_text(encoding="utf-8") + "\npub struct ActionPlan;\n",
                encoding="utf-8",
            )

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("superseded bootstrap authority marker reintroduced", result.stderr)

    def test_provider_owned_planning_cannot_be_reintroduced(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            provider = root / "crates/linura-provider-sdk/src/lib.rs"
            provider.write_text(
                provider.read_text(encoding="utf-8") + "\npub trait Provider: Send + Sync {}\n",
                encoding="utf-8",
            )

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("superseded bootstrap authority marker reintroduced", result.stderr)

    def test_canonical_lifecycle_scaffold_cannot_be_accidentally_deleted(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            lifecycle = root / "crates/linura-lifecycle/src/lib.rs"
            text = lifecycle.read_text(encoding="utf-8").replace(
                "pub const MUTATION_STAGES: [MutationStage; 11]",
                "const REMOVED_MUTATION_STAGES: [MutationStage; 11]",
                1,
            )
            lifecycle.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("future authority scaffold missing", result.stderr)

    def test_narrow_executor_scaffold_cannot_be_accidentally_deleted(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            executor = root / "executors/linura-executor-systemd/src/lib.rs"
            text = executor.read_text(encoding="utf-8").replace(
                "pub enum SystemdOperation",
                "enum RemovedSystemdOperation",
                1,
            )
            executor.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("future authority scaffold missing", result.stderr)


if __name__ == "__main__":
    unittest.main()
