#!/usr/bin/env bash
set -euo pipefail

readonly VERSION_CONTRACT="tools/codex/versions.env"
if [[ ! -f "$VERSION_CONTRACT" ]]; then
  echo "missing Codex toolchain contract: $VERSION_CONTRACT" >&2
  exit 1
fi
# shellcheck disable=SC1090
source "$VERSION_CONTRACT"

: "${RUSTUP_VERSION:?}"
: "${RUST_VERSION:?}"
: "${CARGO_AUDIT_VERSION:?}"
: "${ACTIONLINT_VERSION:?}"
: "${PYTHON_MAJOR_MINOR:?}"
: "${HOST_OS:?}"
: "${HOST_ARCH:?}"

export PATH="${HOME}/.cargo/bin:${HOME}/.local/bin:${PATH}"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    printf 'missing required command: %s\n' "$command_name" >&2
    exit 1
  fi
}

reports_exact_version() {
  local expected="$1"
  shift
  local output token
  if ! output="$("$@" 2>/dev/null)"; then
    return 1
  fi
  while IFS= read -r token; do
    token="${token#v}"
    if [[ "$token" == "$expected" ]]; then
      return 0
    fi
  done < <(printf '%s\n' "$output" | tr '[:space:]' '\n')
  return 1
}

for command_name in bash cargo-audit git python3 rustup uname; do
  require_command "$command_name"
done

actual_os="$(uname -s)"
actual_arch="$(uname -m)"
test "$actual_os" = "$HOST_OS"
test "$actual_arch" = "$HOST_ARCH"

actual_python="$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')"
test "$actual_python" = "$PYTHON_MAJOR_MINOR"

if ! reports_exact_version "$RUSTUP_VERSION" rustup --version; then
  printf 'unexpected rustup version; expected exactly %s (run Codex environment setup)\n' "$RUSTUP_VERSION" >&2
  exit 1
fi

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

# rustc/cargo normally resolve through rustup proxies. Do not invoke those
# proxies until rustup's local state proves the declared toolchain is installed.
installed_rust_toolchain="$(
  rustup toolchain list | awk -v version="$RUST_VERSION" '
    $1 == version || index($1, version "-") == 1 { print $1; exit }
  '
)"
if [[ -z "$installed_rust_toolchain" ]]; then
  printf 'missing required Rust toolchain: %s (run Codex environment setup)\n' "$RUST_VERSION" >&2
  exit 1
fi

rustc_bin="$(rustup which --toolchain "$installed_rust_toolchain" rustc)"
cargo_bin="$(rustup which --toolchain "$installed_rust_toolchain" cargo)"
test -x "$rustc_bin"
test -x "$cargo_bin"

actual_rust="$("$rustc_bin" --version | awk '{print $2}')"
test "$actual_rust" = "$RUST_VERSION"

if ! reports_exact_version "$CARGO_AUDIT_VERSION" cargo-audit --version; then
  printf 'unexpected cargo-audit version; expected exactly %s\n' "$CARGO_AUDIT_VERSION" >&2
  exit 1
fi

actionlint_bin="${HOME}/.local/linura-tools/actionlint/${ACTIONLINT_VERSION}/actionlint"
test -x "$actionlint_bin"
if ! reports_exact_version "$ACTIONLINT_VERSION" "$actionlint_bin" -version; then
  printf 'unexpected actionlint version; expected exactly %s\n' "$ACTIONLINT_VERSION" >&2
  exit 1
fi

# Prove the exact dependency graph is locally available without network mutation.
"$cargo_bin" metadata --locked --offline --format-version 1 >/dev/null

# Validate workflow syntax/contexts with the same pinned semantic validator CI uses.
"$actionlint_bin" -color

# Environment verification must not change tracked source.
git diff --exit-code -- .
git diff --cached --exit-code -- .

if [[ "${1:-}" == "--full" ]]; then
  CARGO_NET_OFFLINE=true "$cargo_bin" xtask check
  git diff --exit-code -- .
  git diff --cached --exit-code -- .
elif [[ $# -gt 0 ]]; then
  echo "usage: bash scripts/preflight_codex_environment.sh [--full]" >&2
  exit 2
fi

printf 'Linura Codex environment preflight passed.\n'
printf '  host: %s-%s\n' "$actual_os" "$actual_arch"
printf '  python: %s\n' "$(python3 --version)"
printf '  rustup: %s\n' "$(rustup --version | head -1)"
printf '  rustc: %s\n' "$("$rustc_bin" --version)"
printf '  cargo-audit: %s\n' "$(cargo-audit --version)"
printf '  actionlint: %s\n' "$("$actionlint_bin" -version | head -1)"
