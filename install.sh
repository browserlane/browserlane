#!/bin/sh
# browserlane installer (macOS / Linux).
#
#   curl -fsSL https://browserlane.com/install.sh | sh
#
# Downloads the latest `bl` release for your platform, verifies its checksum,
# installs it, and puts it on your PATH. Re-run any time to update.
#
# Env overrides:
#   BL_VERSION        install a specific tag (e.g. v0.1.0) instead of latest
#   BL_INSTALL_DIR    install location (default: $HOME/.browserlane/bin)
#   BL_NO_MODIFY_PATH set to skip editing your shell profile
set -eu

REPO="browserlane/browserlane"
BIN="bl"
INSTALL_DIR="${BL_INSTALL_DIR:-$HOME/.browserlane/bin}"

main() {
  need curl
  need tar

  target="$(detect_target)"
  version="${BL_VERSION:-$(latest_version)}"
  [ -n "$version" ] || err "could not determine the latest release (set BL_VERSION to a tag like v0.1.0)"

  asset="${BIN}-${version}-${target}.tar.gz"
  base="https://github.com/${REPO}/releases/download/${version}"
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT

  say "downloading ${asset}"
  curl -fsSL "${base}/${asset}" -o "${tmp}/${asset}" \
    || err "download failed — is ${version} released for ${target}? (${base}/${asset})"

  if curl -fsSL "${base}/SHA256SUMS" -o "${tmp}/SHA256SUMS" 2>/dev/null; then
    verify "$tmp" "$asset"
  else
    say "note: SHA256SUMS not found — skipping checksum"
  fi

  tar -xzf "${tmp}/${asset}" -C "$tmp"
  binpath="$(find "$tmp" -type f -name "$BIN" | head -n1)"
  [ -n "$binpath" ] || err "could not find ${BIN} inside the archive"

  # Install via stage-then-rename, never an in-place overwrite: the target
  # path always gets a FRESH file. On macOS an overwritten binary can inherit a
  # stale Gatekeeper verdict and get SIGKILLed on first run; a new inode gets a
  # clean assessment. The rename is atomic within the same directory.
  mkdir -p "$INSTALL_DIR"
  staged="${INSTALL_DIR}/.${BIN}.new.$$"
  cp "$binpath" "$staged"
  chmod 0755 "$staged"
  mv -f "$staged" "${INSTALL_DIR}/${BIN}"
  say "installed ${BIN} ${version} -> ${INSTALL_DIR}/${BIN}"

  ensure_path
  say "done — run: ${BIN} --version"
}

detect_target() {
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Darwin)
      case "$arch" in
        arm64 | aarch64) echo "aarch64-apple-darwin" ;;
        x86_64) echo "x86_64-apple-darwin" ;;
        *) err "unsupported macOS arch: ${arch}" ;;
      esac
      ;;
    Linux)
      case "$arch" in
        x86_64 | amd64) echo "x86_64-unknown-linux-gnu" ;;
        *) err "no prebuilt Linux binary for ${arch} yet — build from source (see README)" ;;
      esac
      ;;
    *) err "unsupported OS: ${os} — on Windows use install.ps1" ;;
  esac
}

latest_version() {
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
    | grep '"tag_name"' | head -n1 | sed -E 's/.*"tag_name"[ ]*:[ ]*"([^"]+)".*/\1/'
}

verify() {
  d="$1"
  a="$2"
  want="$(awk -v f="$a" '$2 == f {print $1}' "${d}/SHA256SUMS" | head -n1)"
  [ -n "$want" ] || { say "note: ${a} not listed in SHA256SUMS — skipping checksum"; return 0; }
  if command -v sha256sum >/dev/null 2>&1; then
    got="$(sha256sum "${d}/${a}" | awk '{print $1}')"
  else
    got="$(shasum -a 256 "${d}/${a}" | awk '{print $1}')"
  fi
  [ "$want" = "$got" ] || err "checksum mismatch for ${a}"
  say "checksum ok"
}

ensure_path() {
  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) return 0 ;;
  esac
  line="export PATH=\"${INSTALL_DIR}:\$PATH\""
  if [ -n "${BL_NO_MODIFY_PATH:-}" ]; then
    say "add to PATH: ${line}"
    return 0
  fi
  case "${SHELL:-}" in
    *zsh) rc="$HOME/.zshrc" ;;
    *bash) rc="$HOME/.bashrc" ;;
    *) rc="$HOME/.profile" ;;
  esac
  if grep -qsF "$INSTALL_DIR" "$rc" 2>/dev/null; then
    say "PATH already configured in ${rc} — restart your shell"
  else
    printf '\n# browserlane\n%s\n' "$line" >> "$rc" \
      && say "added ${INSTALL_DIR} to PATH in ${rc} — restart your shell or: source ${rc}" \
      || say "add to PATH: ${line}"
  fi
}

need() { command -v "$1" >/dev/null 2>&1 || err "required tool not found: $1"; }
say() { printf 'browserlane: %s\n' "$1" >&2; }
err() { printf 'browserlane: error: %s\n' "$1" >&2; exit 1; }

main "$@"
