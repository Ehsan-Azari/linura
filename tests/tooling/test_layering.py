from __future__ import annotations

from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


class LayeringContractTests(unittest.TestCase):
    def _run_checker(self, root: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(ROOT / "tools/check_layering.py"), str(root)],
            capture_output=True,
            text=True,
            check=False,
        )

    def _copy_fixture(self, destination: Path) -> None:
        for rel in (
            "Cargo.toml",
            "contracts/layering.toml",
            "docs/terminology.md",
            "docs/provider-model.md",
            "docs/state-model.md",
            "crates/linura-provider-sdk/src/lib.rs",
        ):
            source = ROOT / rel
            target = destination / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)

        workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        for member in workspace["workspace"]["members"]:
            source = ROOT / member / "Cargo.toml"
            target = destination / member / "Cargo.toml"
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)

    def _add_dependency(self, manifest: Path, name: str, spec: str) -> None:
        text = manifest.read_text(encoding="utf-8")
        dependency = f"{name} = {spec}\n"
        if "[dependencies]\n" in text:
            text = text.replace("[dependencies]\n", "[dependencies]\n" + dependency, 1)
        elif "[lints]" in text:
            text = text.replace("[lints]", "[dependencies]\n" + dependency + "\n[lints]", 1)
        else:
            text += "\n[dependencies]\n" + dependency
        manifest.write_text(text, encoding="utf-8")

    def test_repository_layering_contract_is_valid(self) -> None:
        result = self._run_checker(ROOT)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_planner_cannot_gain_dbus_transport_dependency(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            manifest = root / "crates/linura-planner/Cargo.toml"
            self._add_dependency(manifest, "zbus", '"5"')

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("linura-planner violates transport-neutral boundary", result.stderr)

    def test_dbus_transport_cannot_own_planner_dependency(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            manifest = root / "crates/linura-dbus/Cargo.toml"
            self._add_dependency(
                manifest,
                "linura-planner",
                '{ path = "../linura-planner" }',
            )

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("linura-dbus violates inward dependency boundary", result.stderr)

    def test_semantic_crate_cannot_depend_on_concrete_executor(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            manifest = root / "crates/linura-core/Cargo.toml"
            self._add_dependency(
                manifest,
                "linura-executor-systemd",
                '{ path = "../../executors/linura-executor-systemd" }',
            )

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("linura-core violates concrete executor/provider boundary", result.stderr)

    def test_observation_control_cannot_gain_concrete_linux_adapter(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            manifest = root / "crates/linura-observation-control/Cargo.toml"
            self._add_dependency(
                manifest,
                "linura-linux-observation",
                '{ path = "../linura-linux-observation" }',
            )

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(
                "linura-observation-control violates inward dependency boundary",
                result.stderr,
            )

    def test_actor_terminology_cannot_be_repurposed_for_backend_workers(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            terminology = root / "docs/terminology.md"
            terminology.write_text(
                terminology.read_text(encoding="utf-8").replace(
                    "**Actor:** authenticated principal requesting an operation.",
                    "**Actor:** reusable backend worker.",
                    1,
                ),
                encoding="utf-8",
            )

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("layering marker missing from docs/terminology.md", result.stderr)

    def test_canonical_observation_marker_cannot_silently_disappear(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._copy_fixture(root)
            sdk = root / "crates/linura-provider-sdk/src/lib.rs"
            sdk.write_text(
                sdk.read_text(encoding="utf-8").replace(
                    "MUST NOT become a second canonical observation model",
                    "may become a second observation model",
                    1,
                ),
                encoding="utf-8",
            )

            result = self._run_checker(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(
                "layering marker missing from crates/linura-provider-sdk/src/lib.rs",
                result.stderr,
            )


if __name__ == "__main__":
    unittest.main()
