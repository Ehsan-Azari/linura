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

    def test_fresh_host_contract_requires_supported_glibc_and_linker(self) -> None:
        for script_name in (
            "scripts/setup_codex_environment.sh",
            "scripts/preflight_codex_environment.sh",
        ):
            script = (ROOT / script_name).read_text(encoding="utf-8")
            host_loop = re.search(
                r"for command_name in (?P<commands>[^;]+); do\n\s+require_command",
                script,
            )
            self.assertIsNotNone(host_loop, script_name)
            assert host_loop is not None
            commands = set(host_loop.group("commands").split())
            self.assertIn("cc", commands, script_name)
            self.assertIn("getconf", commands, script_name)
            self.assertIn("getconf GNU_LIBC_VERSION", script, script_name)
            self.assertIn("MIN_GLIBC_MAJOR=2", script, script_name)
            self.assertIn("MIN_GLIBC_MINOR=17", script, script_name)
            self.assertIn("unsupported glibc version", script, script_name)

    def test_custom_cargo_home_is_the_rust_tool_bin_root(self) -> None:
        for script_name in (
            "scripts/setup_codex_environment.sh",
            "scripts/preflight_codex_environment.sh",
        ):
            script = (ROOT / script_name).read_text(encoding="utf-8")
            self.assertIn(
                'readonly CARGO_ROOT="${CARGO_HOME:-${HOME}/.cargo}"',
                script,
                script_name,
            )
            self.assertIn('export CARGO_HOME="$CARGO_ROOT"', script, script_name)
            self.assertIn('${CARGO_ROOT}/bin:', script, script_name)

    def test_rustup_cannot_self_update_during_pinned_toolchain_install(self) -> None:
        setup = (ROOT / "scripts/setup_codex_environment.sh").read_text(encoding="utf-8")
        disable = "rustup set auto-self-update disable"
        install = (
            'rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal '
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

    def test_rust_toolchain_host_is_fully_qualified_and_verified(self) -> None:
        setup = (ROOT / "scripts/setup_codex_environment.sh").read_text(encoding="utf-8")
        preflight = (ROOT / "scripts/preflight_codex_environment.sh").read_text(encoding="utf-8")
        for script in (setup, preflight):
            self.assertIn(
                'readonly RUST_TOOLCHAIN="${RUST_VERSION}-${RUSTUP_TARGET}"',
                script,
            )
        self.assertIn('rustup set default-host "$RUSTUP_TARGET"', setup)
        self.assertIn('rustup toolchain install "$RUST_TOOLCHAIN"', setup)
        self.assertIn('rustup default "$RUST_TOOLCHAIN"', setup)
        self.assertIn('active_toolchain="$(rustup show active-toolchain', setup)
        self.assertIn('active_toolchain="$(rustup show active-toolchain', preflight)
        self.assertIn('$1 == toolchain', preflight)
        self.assertIn('rustup which --toolchain "$RUST_TOOLCHAIN" rustc', preflight)
        self.assertIn('rustup which --toolchain "$RUST_TOOLCHAIN" cargo', preflight)

    def test_task_preflight_requires_exact_rustup_version(self) -> None:
        preflight = (ROOT / "scripts/preflight_codex_environment.sh").read_text(encoding="utf-8")
        self.assertIn(': "${RUSTUP_VERSION:?}"', preflight)
        self.assertIn('reports_exact_version "$RUSTUP_VERSION" rustup --version', preflight)


if __name__ == "__main__":
    unittest.main()
