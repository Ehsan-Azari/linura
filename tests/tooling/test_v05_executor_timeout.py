from __future__ import annotations

from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
EXECUTOR = ROOT / "executors/linura-executor-systemd/src/lib.rs"


class V05ExecutorTimeoutBoundaryTests(unittest.TestCase):
    def test_systemd_method_calls_are_deadline_bounded(self) -> None:
        source = EXECUTOR.read_text(encoding="utf-8")
        self.assertIn(
            "const SYSTEMD_METHOD_TIMEOUT: Duration = Duration::from_secs(5);",
            source,
        )
        self.assertIn(".method_timeout(SYSTEMD_METHOD_TIMEOUT)", source)

    def test_timeout_after_dispatch_attempt_is_indeterminate(self) -> None:
        source = EXECUTOR.read_text(encoding="utf-8")
        self.assertIn("dispatch_timeout_is_indeterminate_and_bounded", source)
        self.assertIn("ExecutionDisposition::Indeterminate", source)
        self.assertIn("systemd dispatch outcome is indeterminate", source)


if __name__ == "__main__":
    unittest.main()
