from __future__ import annotations

from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
CONTROL = ROOT / "crates/linura-control/src/durable_authority.rs"
TRANSACTION = ROOT / "crates/linura-transaction/src/lib.rs"
SQLITE_STORE = ROOT / "crates/linura-persistence-sqlite/src/store.rs"
SQLITE_SCHEMA = ROOT / "crates/linura-persistence-sqlite/src/schema.rs"
SQLITE_VALIDATION = ROOT / "crates/linura-persistence-sqlite/src/validation.rs"


class V04AuthorityContractTests(unittest.TestCase):
    def test_authority_key_consumes_non_copy_container(self) -> None:
        text = TRANSACTION.read_text(encoding="utf-8")
        self.assertIn("pub fn new(mut bytes: Vec<u8>)", text)
        self.assertNotIn("bytes: [u8; AUTHORITY_MUTATION_KEY_BYTES]", text)
        self.assertIn("validate_authority_key_bytes(&mut bytes)?", text)
        validation = text[
            text.index("fn validate_authority_key_bytes") : text.index(
                "impl TransactionAuthorityKey"
            )
        ]
        self.assertIn("bytes.zeroize();", validation)
        self.assertIn("InvalidAuthorityKey", validation)
        self.assertEqual(text.count("self.bytes.zeroize();"), 3)

    def test_all_sensitive_mutations_are_sealed(self) -> None:
        text = TRANSACTION.read_text(encoding="utf-8")
        self.assertIn("pub fn verify_handoff", text)
        self.assertIn("pub fn verify_recovery", text)
        self.assertIn("pub fn verify_commit", text)
        self.assertIn("authorized_at_unix_ms", text)
        self.assertIn("expires_at_unix_ms", text)
        commit = text[text.index("pub struct CommitRequest"):text.index("pub struct AbortRequest")]
        self.assertNotIn("pub transaction_id:", commit)
        self.assertIn("authority_tag:", commit)

    def test_control_startup_cleanup_and_commit_are_mandatory(self) -> None:
        text = CONTROL.read_text(encoding="utf-8")
        constructor = text[text.index("pub fn new("):text.index("pub fn candidate(")]
        self.assertIn("control.abort_prepared_after_restart()?", constructor)
        self.assertIn("pub fn commit_verified", text)
        self.assertNotIn("pub fn into_store", text)
        self.assertIn("VerifiedDurableAuthority", text)

    def test_sqlite_checks_freshness_under_lock_and_blocks_replace_delete(self) -> None:
        store = SQLITE_STORE.read_text(encoding="utf-8")
        schema = SQLITE_SCHEMA.read_text(encoding="utf-8")
        validation = SQLITE_VALIDATION.read_text(encoding="utf-8")
        self.assertIn("enforce_authority_window", store)
        self.assertIn("verify_commit(request)", store)
        self.assertIn("PRAGMA recursive_triggers=ON", validation)
        self.assertIn("CREATE TRIGGER generations_no_delete", schema)
        self.assertIn("CREATE TRIGGER transactions_no_delete", schema)
        self.assertIn("integrity_tag", schema)


if __name__ == "__main__":
    unittest.main()
