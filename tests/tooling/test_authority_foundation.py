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
            "crates/linura-control/src/policy_review.rs",
            "crates/linura-control/src/risk_classification.rs",
            "crates/linura-policy/src/lib.rs",
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

    def test_unclassified_risk_guard_cannot_be_deleted(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            classifier = root / "crates/linura-control/src/risk_classification.rs"
            text = classifier.read_text(encoding="utf-8").replace(
                "return RiskClassification::Unclassified {",
                "return RiskClassification::NotApplicable {",
                1,
            )
            classifier.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("future authority scaffold missing", result.stderr)
            self.assertIn("return RiskClassification::Unclassified {", result.stderr)

    def test_policy_review_must_keep_downgrade_blocker(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            review = root / "crates/linura-control/src/policy_review.rs"
            text = review.read_text(encoding="utf-8").replace(
                'code: "authority-risk-downgrade-rejected".into()',
                'code: "authority-risk-downgrade-removed".into()',
                1,
            )
            review.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("future authority scaffold missing", result.stderr)
            self.assertIn("authority-risk-downgrade-rejected", result.stderr)

    def test_security_sensitive_approval_class_cannot_be_weakened(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            policy = root / "crates/linura-policy/src/lib.rs"
            strong = (
                "RiskClass::SecuritySensitive => PolicyDecision::RequireApproval {\n"
                "                class: ApprovalClass::Administrator,"
            )
            weak = (
                "RiskClass::SecuritySensitive => PolicyDecision::RequireApproval {\n"
                "                class: ApprovalClass::InteractiveUser,"
            )
            text = policy.read_text(encoding="utf-8")
            self.assertIn(strong, text)
            policy.write_text(text.replace(strong, weak, 1), encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(
                "SecuritySensitive must require exactly ApprovalClass::Administrator",
                result.stderr,
            )

    def test_destructive_approval_class_cannot_be_weakened(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            policy = root / "crates/linura-policy/src/lib.rs"
            strong = (
                "RiskClass::Destructive => PolicyDecision::RequireApproval {\n"
                "                class: ApprovalClass::DestructiveAction,"
            )
            weak = (
                "RiskClass::Destructive => PolicyDecision::RequireApproval {\n"
                "                class: ApprovalClass::InteractiveUser,"
            )
            text = policy.read_text(encoding="utf-8")
            self.assertIn(strong, text)
            policy.write_text(text.replace(strong, weak, 1), encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(
                "Destructive must require exactly ApprovalClass::DestructiveAction",
                result.stderr,
            )

    def test_comment_cannot_spoof_security_approval_mapping(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            policy = root / "crates/linura-policy/src/lib.rs"
            strong = (
                "RiskClass::SecuritySensitive => PolicyDecision::RequireApproval {\n"
                "                class: ApprovalClass::Administrator,"
            )
            weak = (
                "RiskClass::SecuritySensitive => PolicyDecision::RequireApproval {\n"
                "                class: ApprovalClass::InteractiveUser,"
            )
            text = policy.read_text(encoding="utf-8")
            self.assertIn(strong, text)
            text = text.replace(strong, weak, 1) + f"\n/* {strong} */\n"
            policy.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(
                "SecuritySensitive must require exactly ApprovalClass::Administrator",
                result.stderr,
            )

    def test_risk_short_circuit_before_canonical_match_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            policy = root / "crates/linura-policy/src/lib.rs"
            anchor = "        let agent_proposal = subject.actor().kind == ActorKind::Agent;\n"
            bypass = (
                "        if subject.prospective_risk() == RiskClass::SecuritySensitive {\n"
                "            return PolicyDecision::RequireApproval {\n"
                "                class: ApprovalClass::InteractiveUser,\n"
                "                reason: \"unsafe test bypass\".into(),\n"
                "            };\n"
                "        }\n\n"
            )
            text = policy.read_text(encoding="utf-8")
            self.assertIn(anchor, text)
            policy.write_text(text.replace(anchor, bypass + anchor, 1), encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("pre-risk control flow changed", result.stderr)

    def test_actor_short_circuit_before_canonical_match_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            policy = root / "crates/linura-policy/src/lib.rs"
            anchor = "        let agent_proposal = subject.actor().kind == ActorKind::Agent;\n"
            bypass = (
                "        if subject.actor().kind == ActorKind::Agent {\n"
                "            return PolicyDecision::RequireApproval {\n"
                "                class: ApprovalClass::InteractiveUser,\n"
                "                reason: \"unsafe actor shortcut\".into(),\n"
                "            };\n"
                "        }\n\n"
            )
            text = policy.read_text(encoding="utf-8")
            self.assertIn(anchor, text)
            policy.write_text(text.replace(anchor, bypass + anchor, 1), encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("pre-risk control flow changed", result.stderr)

    def test_guarded_protected_risk_arm_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            policy = root / "crates/linura-policy/src/lib.rs"
            anchor = "            RiskClass::SecuritySensitive => PolicyDecision::RequireApproval {\n"
            guarded = (
                "            RiskClass::SecuritySensitive if agent_proposal => {\n"
                "                PolicyDecision::RequireApproval {\n"
                "                    class: ApprovalClass::InteractiveUser,\n"
                "                    reason: \"unsafe guarded shortcut\".into(),\n"
                "                }\n"
                "            },\n"
            )
            text = policy.read_text(encoding="utf-8")
            self.assertIn(anchor, text)
            policy.write_text(text.replace(anchor, guarded + anchor, 1), encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertTrue(
                "exactly four top-level decision arms" in result.stderr
                or "RiskClass::SecuritySensitive exactly once" in result.stderr
            )

    def test_milestone_cannot_drop_risk_floor_contract(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            milestone = root / "docs/milestones/v0.3.0.md"
            text = milestone.read_text(encoding="utf-8").replace(
                "The planner's `prospective_risk` is a risk floor.",
                "Planner risk is advisory.",
                1,
            )
            milestone.write_text(text, encoding="utf-8")

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("v0.3 authority contract marker missing", result.stderr)
            self.assertIn("prospective_risk", result.stderr)


if __name__ == "__main__":
    unittest.main()
