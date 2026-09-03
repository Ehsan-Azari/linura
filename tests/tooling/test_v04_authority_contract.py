from __future__ import annotations

import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
CONTROL = ROOT / "crates/linura-control/src/durable_authority.rs"
TRANSACTION = ROOT / "crates/linura-transaction/src/lib.rs"
SQLITE_STORE = ROOT / "crates/linura-persistence-sqlite/src/store.rs"
SQLITE_SCHEMA = ROOT / "crates/linura-persistence-sqlite/src/schema.rs"
SQLITE_VALIDATION = ROOT / "crates/linura-persistence-sqlite/src/validation.rs"
SQLITE_INTEGRITY = ROOT / "crates/linura-persistence-sqlite/src/integrity.rs"
MIGRATION_DESCRIPTOR = (
    ROOT / "migrations/system/0001-v04-hardened-authority-transactions.json"
)
ADR = ROOT / "docs/adr/0020-sealed-durable-mutation-authority.md"


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
        commit = text[text.index("pub struct CommitRequest") : text.index("pub struct AbortRequest")]
        self.assertNotIn("pub transaction_id:", commit)
        self.assertIn("authority_tag:", commit)

    def test_control_startup_cleanup_and_commit_are_mandatory(self) -> None:
        text = CONTROL.read_text(encoding="utf-8")
        constructor = text[text.index("pub fn new(") : text.index("pub fn candidate(")]
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

    def test_integrity_key_uses_non_elidable_atomic_scrubbing(self) -> None:
        text = SQLITE_INTEGRITY.read_text(encoding="utf-8")
        self.assertIn("AtomicU8::from_mut(byte).store(0, Ordering::SeqCst)", text)
        self.assertIn("validate_integrity_key_bytes(&mut bytes)?", text)
        self.assertNotIn("self.bytes.fill(0)", text)

    def test_authority_store_migration_is_registered_and_ledger_verified(self) -> None:
        descriptor = json.loads(MIGRATION_DESCRIPTOR.read_text(encoding="utf-8"))
        schema = SQLITE_SCHEMA.read_text(encoding="utf-8")
        validation = SQLITE_VALIDATION.read_text(encoding="utf-8")
        self.assertEqual(descriptor["schema_version"], 1)
        self.assertEqual(descriptor["id"], "0001-v04-hardened-authority-transactions")
        self.assertEqual(descriptor["introduced_in"], "0.4.0")
        self.assertEqual(descriptor["scope"], "system")
        self.assertFalse(descriptor["reversible"])
        self.assertFalse(descriptor["requires_snapshot"])
        self.assertIn("sqlite.application_id == 0", descriptor["preconditions"])
        self.assertIn("sqlite.user_version == 0", descriptor["preconditions"])
        self.assertIn(
            "sqlite_schema contains no non-SQLite application objects",
            descriptor["preconditions"],
        )
        verification = descriptor["verification"]
        self.assertEqual(verification["ledger"], "schema_migrations")
        self.assertEqual(
            verification["ledger_id"], "0001-v04-hardened-authority-transactions"
        )
        self.assertEqual(verification["checksum_domain"], "linura.sqlite.migration.v1")
        self.assertIn(
            'MIGRATION_ID: &str = "0001-v04-hardened-authority-transactions"',
            schema,
        )
        self.assertIn("SELECT length(CAST(checksum AS BLOB))", validation)
        self.assertIn("migration_checksum()", validation)

    def test_adr_describes_keyed_tamper_detection_not_sql_gate(self) -> None:
        text = ADR.read_text(encoding="utf-8")
        lower = text.lower()
        self.assertNotIn("linura_internal_mutation_gate", text)
        self.assertIn("raw sqlite writes can physically alter", lower)
        self.assertIn("record-integrity", lower)
        self.assertIn("coherent rollback", lower)


if __name__ == "__main__":
    unittest.main()
