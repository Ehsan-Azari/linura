from __future__ import annotations

import hashlib
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
