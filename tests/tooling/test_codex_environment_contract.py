from __future__ import annotations

from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


class CodexEnvironmentContractTests(unittest.TestCase):
    def test_rustup_bootstrap_is_pinned_and_digest_verified(self) -> None:
        versions = (ROOT / "tools/codex/versions.env").read_text(encoding="utf-8")
        setup = (ROOT / "scripts/setup_codex_environment.sh").read_text(encoding="utf-8")

        self.assertRegex(versions, r"(?m)^RUSTUP_VERSION=\d+\.\d+\.\d+$")
        self.assertRegex(versions, r"(?m)^RUSTUP_INIT_SHA256=[0-9a-f]{64}$")
        self.assertIn(
            'https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/${RUSTUP_TARGET}/rustup-init',
            setup,
        )
        self.assertIn(
            'printf \'%s  %s\\n\' "$RUSTUP_INIT_SHA256" "$rustup_init_path" | sha256sum --check --strict',
            setup,
        )
        self.assertIn("--default-toolchain none", setup)

    def test_fresh_host_does_not_require_preinstalled_rustup(self) -> None:
        setup = (ROOT / "scripts/setup_codex_environment.sh").read_text(encoding="utf-8")
        host_loop = re.search(
            r"for command_name in (?P<commands>[^;]+); do\n\s+require_command",
            setup,
        )
        self.assertIsNotNone(host_loop)
        assert host_loop is not None
        self.assertNotIn("rustup", host_loop.group("commands").split())
        self.assertIn("if ! command -v rustup", setup)

    def test_rustup_cannot_self_update_during_pinned_toolchain_install(self) -> None:
        setup = (ROOT / "scripts/setup_codex_environment.sh").read_text(encoding="utf-8")
        disable = 'rustup set auto-self-update disable'
        install = (
            'rustup toolchain install "$RUST_VERSION" --profile minimal '
            '--component clippy --component rustfmt --no-self-update'
        )
        self.assertIn(disable, setup)
        self.assertIn(install, setup)
        self.assertLess(setup.index(disable), setup.index(install))
        self.assertGreaterEqual(
            setup.count('reports_exact_version "$RUSTUP_VERSION" rustup --version'),
            2,
        )
        self.assertIn("rustup self-updated during toolchain setup", setup)

    def test_task_preflight_requires_exact_rustup_version(self) -> None:
        preflight = (ROOT / "scripts/preflight_codex_environment.sh").read_text(encoding="utf-8")
        self.assertIn(': "${RUSTUP_VERSION:?}"', preflight)
        self.assertIn('reports_exact_version "$RUSTUP_VERSION" rustup --version', preflight)


if __name__ == "__main__":
    unittest.main()
