from __future__ import annotations

from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
CONTROL = ROOT / "crates/linura-control/src/durable_authority.rs"
TRANSACTION = ROOT / "crates/linura-transaction/src/lib.rs"
TRANSACTION_CARGO = ROOT / "crates/linura-transaction/Cargo.toml"


class V04AuthorityContractTests(unittest.TestCase):
    def test_handoff_is_bound_to_current_authenticated_principal(self) -> None:
        text = CONTROL.read_text(encoding="utf-8")
        self.assertIn("principal: &AuthenticatedPrincipal", text)
        self.assertIn("principal.as_str() != prepared.candidate.principal.as_str()", text)
        self.assertIn("principal.as_str() != prepared.binding.principal().as_str()", text)
        self.assertIn("principal.as_str() != prepared.snapshot.principal.as_str()", text)

    def test_terminal_recovery_rechecks_freshness_before_sealing(self) -> None:
        text = CONTROL.read_text(encoding="utf-8")
        no_change = text.index("linura_planner::PlanStatus::NoChange => {")
        change = text.index("linura_planner::PlanStatus::ChangeProposed => {", no_change)
        no_change_block = text[no_change:change]
        freshness = "require_current(self.authority_now_unix_ms()?)"
        self.assertIn(freshness, no_change_block)
        self.assertLess(no_change_block.index(freshness), no_change_block.index("authorize_recovery"))

        precondition = "if precondition_digest != anchor.precondition_digest {"
        conflict = text.index(precondition, change)
        conflict_block = text[conflict:text.index("return match outcome", conflict)]
        self.assertIn(freshness, conflict_block)
        self.assertLess(conflict_block.index(freshness), conflict_block.index("authorize_recovery"))

    def test_authority_secret_holders_zeroize_on_drop(self) -> None:
        cargo = TRANSACTION_CARGO.read_text(encoding="utf-8")
        text = TRANSACTION.read_text(encoding="utf-8")
        self.assertIn('zeroize = "1.8"', cargo)
        self.assertIn("use zeroize::Zeroize;", text)
        self.assertEqual(text.count("self.bytes.zeroize();"), 3)
        self.assertNotIn("self.bytes.fill(0);", text)


if __name__ == "__main__":
    unittest.main()
