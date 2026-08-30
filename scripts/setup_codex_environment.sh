#!/usr/bin/env bash
set -euo pipefail

# Canonical deterministic setup for Linura Codex/cloud development environments.
# This script intentionally does not apt-install mutable "latest" packages.
# Host primitives must be supplied by the configured Codex base environment.

readonly RUST_VERSION="1.98.0"
readonly CARGO_AUDIT_VERSION="0.22.2"
readonly ACTIONLINT_VERSION="1.7.12"
readonly ACTIONLINT_SHA256="8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8"
readonly ACTIONLINT_ARCHIVE="actionlint_${ACTIONLINT_VERSION}_linux_amd64.tar.gz"
readonly ACTIONLINT_URL="https://github.com/rhysd/actionlint/releases/download/v${ACTIONLINT_VERSION}/${ACTIONLINT_ARCHIVE}"
readonly TOOL_ROOT="${HOME}/.local/linura-tools"
readonly ACTIONLINT_ROOT="${TOOL_ROOT}/actionlint/${ACTIONLINT_VERSION}"
readonly BIN_ROOT="${HOME}/.local/bin"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    printf 'missing required host command: %s\n' "$command_name" >&2
    exit 1
  fi
}

for command_name in bash cargo curl git python3 rustc rustup sha256sum tar; do
  require_command "$command_name"
done

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) ;;
  *)
    printf 'unsupported Codex setup host: %s-%s; expected Linux-x86_64\n' "$(uname -s)" "$(uname -m)" >&2
    exit 1
    ;;
esac

mkdir -p "$ACTIONLINT_ROOT" "$BIN_ROOT"

# Install the repository-pinned Rust toolchain. rust-toolchain.toml is the source
# of truth; the explicit value here is cross-checked so the setup cannot drift.
repo_rust_version="$(python3 - <<'PY'
import pathlib
import re

text = pathlib.Path('rust-toolchain.toml').read_text(encoding='utf-8')
match = re.search(r'^channel\s*=\s*"([^"]+)"\s*$', text, re.MULTILINE)
if match is None:
    raise SystemExit('rust-toolchain.toml does not declare a channel')
print(match.group(1))
PY
)"
if [[ "$repo_rust_version" != "$RUST_VERSION" ]]; then
  printf 'setup/toolchain drift: script=%s rust-toolchain.toml=%s\n' "$RUST_VERSION" "$repo_rust_version" >&2
  exit 1
fi

rustup toolchain install "$RUST_VERSION" --profile minimal --component clippy --component rustfmt
rustup default "$RUST_VERSION"

actual_rust="$(rustc --version | awk '{print $2}')"
if [[ "$actual_rust" != "$RUST_VERSION" ]]; then
  printf 'unexpected rustc version: %s (expected %s)\n' "$actual_rust" "$RUST_VERSION" >&2
  exit 1
fi

# Keep the security tool exact. --locked consumes cargo-audit's published lockfile.
if ! command -v cargo-audit >/dev/null 2>&1 || ! cargo-audit --version | grep -Fq " $CARGO_AUDIT_VERSION"; then
  cargo install cargo-audit --locked --version "$CARGO_AUDIT_VERSION"
fi
cargo-audit --version | grep -F " $CARGO_AUDIT_VERSION" >/dev/null

# Install actionlint from the exact archive CI trusts, verified before extraction.
actionlint_bin="$ACTIONLINT_ROOT/actionlint"
if [[ ! -x "$actionlint_bin" ]] || [[ "$($actionlint_bin -version 2>/dev/null || true)" != *"${ACTIONLINT_VERSION}"* ]]; then
  archive_path="$(mktemp)"
  trap 'rm -f "${archive_path:-}"' EXIT
  curl --fail --location --proto '=https' --tlsv1.2 --retry 3 \
    --output "$archive_path" \
    "$ACTIONLINT_URL"
  printf '%s  %s\n' "$ACTIONLINT_SHA256" "$archive_path" | sha256sum --check --strict
  rm -rf "$ACTIONLINT_ROOT"
  mkdir -p "$ACTIONLINT_ROOT"
  tar -xzf "$archive_path" -C "$ACTIONLINT_ROOT" actionlint
  chmod 0755 "$actionlint_bin"
fi
ln -sfn "$actionlint_bin" "$BIN_ROOT/actionlint"
"$BIN_ROOT/actionlint" -version | grep -F "$ACTIONLINT_VERSION" >/dev/null

# Fetch Rust dependencies without changing the lockfile. This warms Codex's setup
# cache while preserving the same dependency graph used by CI/release proof.
cargo fetch --locked

# The setup itself must leave tracked repository state unchanged.
git diff --exit-code -- .
git diff --cached --exit-code -- .

printf 'Linura Codex environment ready.\n'
printf '  rustc: %s\n' "$(rustc --version)"
printf '  cargo: %s\n' "$(cargo --version)"
printf '  cargo-audit: %s\n' "$(cargo-audit --version)"
printf '  actionlint: %s\n' "$("$BIN_ROOT/actionlint" -version | head -1)"
printf 'Run scripts/preflight_codex_environment.sh before delegated implementation.\n'
