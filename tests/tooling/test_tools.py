from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ToolingTests(unittest.TestCase):
    def run_tool(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(args, cwd=ROOT, text=True, capture_output=True, check=False)

    def test_asset_validation(self) -> None:
        result = self.run_tool("python3", "scripts/validate_assets.py")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_acceptance_scenarios_are_discoverable(self) -> None:
        result = self.run_tool("python3", "tools/acceptance.py", "list")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("security-baseline", result.stdout)
        self.assertIn("recovery-native-path", result.stdout)
        self.assertIn("authoritative-observation", result.stdout)

    def test_acceptance_scenario_commands_are_valid_bash(self) -> None:
        scenario_ids: set[str] = set()
        for path in sorted((ROOT / "tests/acceptance").glob("*.json")):
            scenario = json.loads(path.read_text(encoding="utf-8"))
            self.assertEqual(scenario.get("schema_version"), 1, path)
            scenario_id = scenario.get("id")
            self.assertIsInstance(scenario_id, str, path)
            assert isinstance(scenario_id, str)
            self.assertTrue(scenario_id, path)
            self.assertNotIn(scenario_id, scenario_ids, f"duplicate scenario id: {scenario_id}")
            scenario_ids.add(scenario_id)

            steps = scenario.get("steps")
            self.assertIsInstance(steps, list, path)
            assert isinstance(steps, list)
            self.assertTrue(steps, path)
            step_names: set[str] = set()
            for step in steps:
                self.assertIsInstance(step, dict, path)
                assert isinstance(step, dict)
                self.assertEqual(set(step), {"name", "command"}, path)
                name = step.get("name")
                command = step.get("command")
                self.assertIsInstance(name, str, path)
                self.assertIsInstance(command, str, path)
                assert isinstance(name, str)
                assert isinstance(command, str)
                self.assertTrue(name, path)
                self.assertTrue(command, path)
                self.assertNotIn(name, step_names, f"duplicate step name in {scenario_id}: {name}")
                step_names.add(name)

                result = self.run_tool("bash", "-n", "-c", command)
                self.assertEqual(
                    result.returncode,
                    0,
                    f"invalid bash in {path.name}:{name}: {result.stderr}",
                )

    def test_vm_plan_is_available_without_qemu(self) -> None:
        result = self.run_tool("python3", "tools/vm.py", "plan", "--image", "/tmp/linura.qcow2")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("qemu-system-x86_64", result.stdout)
        self.assertIn("linura.qcow2", result.stdout)

    def test_image_plan_is_available_without_mkarchiso(self) -> None:
        result = self.run_tool("python3", "tools/image.py", "plan")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("mkarchiso", result.stdout)

    def test_release_manifest_verification(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            asset = root / "linurad"
            asset.write_bytes(b"linura")
            digest = hashlib.sha256(asset.read_bytes()).hexdigest()
            (root / "SHA256SUMS").write_text(f"{digest}  linurad\n", encoding="utf-8")
            result = self.run_tool("python3", "tools/release_verify.py", str(root))
            self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
