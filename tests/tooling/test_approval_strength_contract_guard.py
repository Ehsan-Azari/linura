from __future__ import annotations

from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / "tools/check_approval_strength_contract.py"
SEMANTIC_TEST = Path("crates/linura-policy/tests/approval_strength.rs")


class ApprovalStrengthContractGuardTests(unittest.TestCase):
    def _run_checker(self, root: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(CHECKER), str(root)],
            capture_output=True,
            text=True,
            check=False,
        )

    def _copy_fixture(self, destination: Path) -> Path:
        source = ROOT / SEMANTIC_TEST
        target = destination / SEMANTIC_TEST
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        return target

    def test_repository_approval_strength_contract_is_guarded(self) -> None:
        result = self._run_checker(ROOT)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_runtime_class_comparison_cannot_be_replaced_with_oracle_self_comparison(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            semantic_test = self._copy_fixture(root)
            text = semantic_test.read_text(encoding="utf-8").replace(
                "class, expected_class,",
                "expected_class, expected_class,",
                1,
            )
            semantic_test.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("evaluated runtime `class`", result.stderr)

    def test_runtime_class_comparison_cannot_be_replaced_with_runtime_self_comparison(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            semantic_test = self._copy_fixture(root)
            text = semantic_test.read_text(encoding="utf-8").replace(
                "class, expected_class,",
                "class, class,",
                1,
            )
            semantic_test.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("evaluated runtime `class`", result.stderr)

    def test_comment_decoy_cannot_restore_gutted_class_comparison(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            semantic_test = self._copy_fixture(root)
            text = semantic_test.read_text(encoding="utf-8").replace(
                "class, expected_class,",
                "class, class,",
                1,
            )
            text = text.replace(
                "fn remote_actor_cannot_use_protected_approval_path()",
                "// assert_eq!(class, expected_class,\nfn remote_actor_cannot_use_protected_approval_path()",
                1,
            )
            semantic_test.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("evaluated runtime `class`", result.stderr)

    def test_string_decoy_cannot_restore_gutted_class_comparison(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            semantic_test = self._copy_fixture(root)
            text = semantic_test.read_text(encoding="utf-8").replace(
                "class, expected_class,",
                "expected_class, expected_class,",
                1,
            )
            text = text.replace(
                "let policy = BaselinePolicy::default();",
                'let _decoy = "assert_eq!(class, expected_class,";\n    let policy = BaselinePolicy::default();',
                1,
            )
            semantic_test.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("evaluated runtime `class`", result.stderr)

    def test_comment_decoy_cannot_restore_removed_runtime_binding(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            semantic_test = self._copy_fixture(root)
            text = semantic_test.read_text(encoding="utf-8").replace(
                "if let PolicyDecision::RequireApproval { class, .. } = decision {",
                "if let PolicyDecision::RequireApproval { class: _, .. } = decision {",
                1,
            )
            text = text.replace(
                "fn remote_actor_cannot_use_protected_approval_path()",
                "// if let PolicyDecision::RequireApproval { class, .. } = decision\nfn remote_actor_cannot_use_protected_approval_path()",
                1,
            )
            semantic_test.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("RequireApproval decision", result.stderr)


if __name__ == "__main__":
    unittest.main()
