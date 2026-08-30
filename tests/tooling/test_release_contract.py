from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "tools" / "release_contract.py"
SPEC = importlib.util.spec_from_file_location("release_contract", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("unable to load release_contract module")
release_contract = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release_contract)


def valid_notes() -> str:
    headings = "\n\n".join(
        f"{heading}\n\nEvidence text."
        for heading in release_contract.REQUIRED_HEADINGS
    )
    return (
        "# v0.0.0 — test release\n\n"
        "**Status:** implementation complete\n"
        "**Claim class:** Architecture\n"
        "**Supported platform profiles:** none\n\n"
        f"{headings}\n\n"
        "Trace link: "
        "https://github.com/linura-org/linura/commit/"
        "0123456789abcdef0123456789abcdef01234567\n"
    )


class ReleaseContractTests(unittest.TestCase):
    def test_contract_requires_canonical_sections_and_traceability(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            notes = Path(temp_dir) / "v0.0.0.md"
            notes.write_text(valid_notes(), encoding="utf-8")
            metadata = release_contract.validate_contract(notes, "v0.0.0")
            self.assertEqual(metadata["claim_class"], "Architecture")
            self.assertEqual(
                metadata["commits"],
                ["0123456789abcdef0123456789abcdef01234567"],
            )

    def test_contract_rejects_product_name_in_release_notes_heading(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            notes = Path(temp_dir) / "v0.0.0.md"
            notes.write_text(
                valid_notes().replace("# v0.0.0 —", "# Linura v0.0.0 —", 1),
                encoding="utf-8",
            )
            with self.assertRaises(release_contract.ContractError):
                release_contract.validate_contract(notes, "v0.0.0")

    def test_contract_rejects_empty_release_theme(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            notes = Path(temp_dir) / "v0.0.0.md"
            notes.write_text(
                valid_notes().replace("# v0.0.0 — test release", "# v0.0.0 — ", 1),
                encoding="utf-8",
            )
            with self.assertRaises(release_contract.ContractError):
                release_contract.validate_contract(notes, "v0.0.0")

    def test_contract_rejects_missing_heading(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            notes = Path(temp_dir) / "v0.0.0.md"
            notes.write_text(
                valid_notes().replace("## Recovery and rollback", "## Recovery"),
                encoding="utf-8",
            )
            with self.assertRaises(release_contract.ContractError):
                release_contract.validate_contract(notes, "v0.0.0")

    def test_evidence_detects_artifact_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            directory = Path(temp_dir)
            notes = directory / "RELEASE_NOTES.md"
            notes.write_text(valid_notes(), encoding="utf-8")
            artifact = directory / "linurad"
            artifact.write_bytes(b"original")
            evidence = release_contract.build_evidence(notes, "v0.0.0", "a" * 40, [artifact])
            (directory / "RELEASE-EVIDENCE.json").write_text(json.dumps(evidence) + "\n", encoding="utf-8")
            release_contract.verify_evidence(directory, "v0.0.0", "a" * 40)
            artifact.write_bytes(b"tampered")
            with self.assertRaises(release_contract.ContractError):
                release_contract.verify_evidence(directory, "v0.0.0", "a" * 40)

    def test_release_tree_uses_filename_as_tag(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            directory = Path(temp_dir)
            (directory / "v0.0.0.md").write_text(valid_notes(), encoding="utf-8")
            release_contract.validate_tree(directory)

    def test_release_body_comparison_normalizes_final_newline(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            directory = Path(temp_dir)
            notes = directory / "notes.md"
            body = directory / "body.md"
            notes.write_text("same\n", encoding="utf-8")
            body.write_text("same\n\n", encoding="utf-8")
            release_contract.compare_release_body(notes, body)


if __name__ == "__main__":
    unittest.main()
