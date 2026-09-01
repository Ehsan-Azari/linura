from __future__ import annotations

from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class RoadmapContractTests(unittest.TestCase):
    def _run_checker(self, root: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(ROOT / "tools/check_roadmap.py"), str(root)],
            capture_output=True,
            text=True,
            check=False,
        )

    def _copy_fixture(self, destination: Path) -> None:
        paths = (
            "contracts/roadmap.toml",
            "docs/roadmap.md",
            "docs/system-domains.md",
            "docs/releases/v0.0.0.md",
            "docs/releases/v0.1.0.md",
            "docs/releases/v0.2.0.md",
            "docs/qualification/v0.1.0.md",
            "docs/qualification/v0.2.0.md",
        )
        for rel in paths:
            source = ROOT / rel
            target = destination / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)

    def test_repository_roadmap_contract_is_valid(self) -> None:
        result = self._run_checker(ROOT)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_current_release_cannot_silently_move_backward(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            contract = root / "contracts/roadmap.toml"
            text = contract.read_text(encoding="utf-8").replace(
                'current_release = "v0.2.0"',
                'current_release = "v0.1.0"',
                1,
            )
            contract.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("current_release must equal", result.stderr)

    def test_document_heading_cannot_drift_from_machine_contract(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            roadmap = root / "docs/roadmap.md"
            text = roadmap.read_text(encoding="utf-8").replace(
                "## v0.3.0 — policy, authorization, approval, and plan review",
                "## v0.3.0 — generic execution",
                1,
            )
            roadmap.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("canonical roadmap missing exact heading", result.stderr)

    def test_supported_mutation_cannot_move_before_complete_lifecycle(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            contract = root / "contracts/roadmap.toml"
            text = contract.read_text(encoding="utf-8")
            old = (
                'version = "v0.3.0"\n'
                'title = "policy, authorization, approval, and plan review"\n'
                'status = "planned"\n'
                'claim_class = "Experimental"\n'
                'depends_on = ["v0.2.0"]\n'
                'durable_recovery = false\n'
                'executor_state = "none"\n'
                'complete_lifecycle = false\n'
                'managed_mutation_support = "none"'
            )
            new = old.replace(
                'managed_mutation_support = "none"',
                'managed_mutation_support = "narrow-experimental"',
            )
            self.assertIn(old, text)
            contract.write_text(text.replace(old, new, 1), encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("architectural gate changed", result.stderr)
            self.assertIn("supported managed mutation requires complete lifecycle proof", result.stderr)

    def test_v05_executor_qualification_cannot_self_promote_to_supported_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            contract = root / "contracts/roadmap.toml"
            text = contract.read_text(encoding="utf-8")
            old = (
                'version = "v0.5.0"\n'
                'title = "first narrow privileged executor and independent verifier"\n'
                'status = "planned"\n'
                'claim_class = "Experimental"\n'
                'depends_on = ["v0.4.0"]\n'
                'durable_recovery = true\n'
                'executor_state = "isolated-qualified"\n'
                'complete_lifecycle = false\n'
                'managed_mutation_support = "none"'
            )
            new = old.replace(
                'managed_mutation_support = "none"',
                'managed_mutation_support = "narrow-experimental"',
            )
            self.assertIn(old, text)
            contract.write_text(text.replace(old, new, 1), encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("architectural gate changed", result.stderr)
            self.assertIn("supported managed mutation requires complete lifecycle proof", result.stderr)

    def test_product_stability_cannot_be_promoted_by_roadmap_only(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            contract = root / "contracts/roadmap.toml"
            text = contract.read_text(encoding="utf-8").replace(
                'product_stability = "experimental"',
                'product_stability = "stable"',
                1,
            )
            contract.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("product_stability must remain experimental", result.stderr)

    def test_machine_contract_cannot_redefine_canonical_lifecycle(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            contract = root / "contracts/roadmap.toml"
            text = contract.read_text(encoding="utf-8").replace(
                "request/intent → observe → plan → validate → authorize → prepare → execute → verify → commit → audit → reconcile",
                "request/intent → plan → execute",
                1,
            )
            contract.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("canonical_lifecycle changed", result.stderr)

    def test_canonical_eleven_stage_lifecycle_cannot_silently_drift_in_docs(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            roadmap = root / "docs/roadmap.md"
            text = roadmap.read_text(encoding="utf-8").replace(
                "request/intent → observe → plan → validate → authorize → prepare → execute → verify → commit → audit → reconcile",
                "request/intent → plan → execute",
                1,
            )
            roadmap.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("canonical roadmap missing governance marker", result.stderr)

    def test_vm_management_cannot_be_confused_with_vm_test_infrastructure(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            domains = root / "docs/system-domains.md"
            text = domains.read_text(encoding="utf-8").replace(
                "test infrastructure, not a product virtualization capability",
                "supported VM product capability",
                1,
            )
            domains.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("system domain map missing virtualization boundary marker", result.stderr)

    def test_v1_does_not_imply_stable(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            contract = root / "contracts/roadmap.toml"
            text = contract.read_text(encoding="utf-8")
            marker = (
                'version = "v1.0.0"\n'
                'title = "meaningful end-user Experimental Linura"\n'
                'status = "planned"\n'
                'claim_class = "Experimental"'
            )
            replacement = marker.replace('claim_class = "Experimental"', 'claim_class = "Stable"')
            self.assertIn(marker, text)
            contract.write_text(text.replace(marker, replacement, 1), encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("v1.0.0 must remain Experimental", result.stderr)


if __name__ == "__main__":
    unittest.main()
