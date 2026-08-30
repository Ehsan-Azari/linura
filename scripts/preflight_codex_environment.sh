#!/usr/bin/env bash
set -euo pipefail

readonly VERSION_CONTRACT="tools/codex/versions.env"
if [[ ! -f "$VERSION_CONTRACT" ]]; then
  echo "missing Codex toolchain contract: $VERSION_CONTRACT" >&2
  exit 1
fi
# shellcheck disable=SC1090
source "$VERSION_CONTRACT"

: "${RUST_VERSION:?}"
: "${CARGO_AUDIT_VERSION:?}"
: "${ACTIONLINT_VERSION:?}"
: "${PYTHON_MAJOR_MINOR:?}"
: "${HOST_OS:?}"
: "${HOST_ARCH:?}"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    printf 'missing required command: %s\n' "$command_name" >&2
    exit 1
  fi
}

for command_name in bash cargo cargo-audit git python3 rustc rustup uname; do
  require_command "$command_name"
done

actual_os="$(uname -s)"
actual_arch="$(uname -m)"
test "$actual_os" = "$HOST_OS"
test "$actual_arch" = "$HOST_ARCH"

actual_python="$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')"
test "$actual_python" = "$PYTHON_MAJOR_MINOR"

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
test "$repo_rust_version" = "$RUST_VERSION"
test "$(rustc --version | awk '{print $2}')" = "$RUST_VERSION"
cargo-audit --version | grep -F " $CARGO_AUDIT_VERSION" >/dev/null

actionlint_bin="${HOME}/.local/linura-tools/actionlint/${ACTIONLINT_VERSION}/actionlint"
test -x "$actionlint_bin"
"$actionlint_bin" -version | grep -F "$ACTIONLINT_VERSION" >/dev/null

# Prove the exact dependency graph is locally available without network mutation.
cargo metadata --locked --offline --format-version 1 >/dev/null

# Validate workflow syntax/contexts with the same pinned semantic validator CI uses.
"$actionlint_bin" -color

# Environment verification must not change tracked source.
git diff --exit-code -- .
git diff --cached --exit-code -- .

if [[ "${1:-}" == "--full" ]]; then
  cargo xtask check
  git diff --exit-code -- .
  git diff --cached --exit-code -- .
elif [[ $# -gt 0 ]]; then
  echo "usage: bash scripts/preflight_codex_environment.sh [--full]" >&2
  exit 2
fi

printf 'Linura Codex environment preflight passed.\n'
printf '  host: %s-%s\n' "$actual_os" "$actual_arch"
printf '  python: %s\n' "$(python3 --version)"
printf '  rustc: %s\n' "$(rustc --version)"
printf '  cargo-audit: %s\n' "$(cargo-audit --version)"
printf '  actionlint: %s\n' "$("$actionlint_bin" -version | head -1)"
