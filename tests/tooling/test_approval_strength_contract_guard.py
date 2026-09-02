from __future__ import annotations

from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / "tools/check_approval_strength_contract.py"


class ApprovalStrengthContractGuardTests(unittest.TestCase):
    def test_production_approval_strength_mutations_are_killed(self) -> None:
        result = subprocess.run(
            [sys.executable, str(CHECKER), str(ROOT)],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, f"{result.stdout}\n{result.stderr}")
        self.assertIn("approval-strength mutation contract passed", result.stdout)


if __name__ == "__main__":
    unittest.main()
