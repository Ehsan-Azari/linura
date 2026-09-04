from __future__ import annotations

import hashlib
import re
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCHEMA = ROOT / "crates/linura-persistence-sqlite/src/schema.rs"


def migration_body(name: str) -> bytes:
    text = SCHEMA.read_text(encoding="utf-8")
    match = re.search(
        rf'pub\(crate\) const {name}: &str = r#"(?P<body>.*?)"#;',
        text,
        flags=re.DOTALL,
    )
    if match is None:
        raise AssertionError(f"missing {name}")
    return match.group("body").encode("utf-8")


class V04MigrationPinTests(unittest.TestCase):
    def test_released_migration_bodies_are_pinned(self) -> None:
        actual_v1 = hashlib.sha256(migration_body("MIGRATION_V1")).hexdigest()
        actual_v2 = hashlib.sha256(migration_body("MIGRATION_V2")).hexdigest()
        expected_v1 = "1e035f128ce2e0a9aa98dede361b25e1b91e7417b2bcba74fc518d68332e4f01"
        expected_v2 = "a9db67a4b967f8d9d84fd2a635bde65dab575ee0a90416b8ec185965e8371a55"
        self.assertEqual(actual_v1, expected_v1, f"pin MIGRATION_V1 as {actual_v1}")
        self.assertEqual(actual_v2, expected_v2, f"pin MIGRATION_V2 as {actual_v2}")

    def test_terminal_release_is_always_post_commit(self) -> None:
        body = migration_body("MIGRATION_V2").decode("utf-8")
        self.assertIn("SELECT 1;", body)
        self.assertNotIn("linura_fs_release_slots", body)


if __name__ == "__main__":
    unittest.main()
