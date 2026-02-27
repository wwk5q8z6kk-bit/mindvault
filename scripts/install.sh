#!/usr/bin/env bash
# MindVault installer — downloads a pre-built release binary.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/wwk5q8z6kk-bit/mindvault/main/scripts/install.sh | bash
#   curl -fsSL ... | bash -s -- --version v0.1.0
#   curl -fsSL ... | bash -s -- --prefix /usr/local
#
# Environment:
#   MINDVAULT_VERSION   — override release tag (e.g. "v0.1.0")
#   MINDVAULT_PREFIX    — install prefix (default: /usr/local)
#   MINDVAULT_NO_MODIFY_PATH — skip PATH hint when installing outside /usr/local

set -euo pipefail

REPO="wwk5q8z6kk-bit/mindvault"
API_BASE="https://api.github.com/repos/${REPO}"
DOWNLOAD_BASE="https://github.com/${REPO}/releases/download"

# --- defaults ---
VERSION="${MINDVAULT_VERSION:-latest}"
PREFIX="${MINDVAULT_PREFIX:-/usr/local}"
NO_MODIFY_PATH="${MINDVAULT_NO_MODIFY_PATH:-}"

# --- arg parsing ---
while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)  VERSION="$2"; shift 2 ;;
    --prefix)   PREFIX="$2"; shift 2 ;;
    --help|-h)
      echo "Usage: install.sh [--version TAG] [--prefix DIR]"
      exit 0 ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

# --- platform detection ---
detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)  os="linux" ;;
    Darwin) os="darwin" ;;
    *)      echo "Unsupported OS: $os" >&2; exit 1 ;;
  esac

  case "$arch" in
    x86_64|amd64)  arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *)             echo "Unsupported architecture: $arch" >&2; exit 1 ;;
  esac

  echo "${os}-${arch}"
}

# --- resolve version ---
resolve_version() {
  if [[ "$VERSION" == "latest" ]]; then
    VERSION="$(curl -fsSL "${API_BASE}/releases/latest" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"//;s/".*//')"
    if [[ -z "$VERSION" ]]; then
      echo "Error: could not resolve latest release tag" >&2
      exit 1
    fi
  fi
  echo "$VERSION"
}

# --- main ---
main() {
  local platform version archive_name url bin_dir tmp_dir

  platform="$(detect_platform)"
  version="$(resolve_version)"
  archive_name="mindvault-${version}-${platform}.tar.gz"
  url="${DOWNLOAD_BASE}/${version}/${archive_name}"
  bin_dir="${PREFIX}/bin"
  tmp_dir="$(mktemp -d)"

  echo "Installing MindVault ${version} for ${platform}..."
  echo "  Archive:  ${url}"
  echo "  Prefix:   ${PREFIX}"

  # download
  if ! curl -fSL --progress-bar -o "${tmp_dir}/${archive_name}" "$url"; then
    echo ""
    echo "Error: download failed. Check that version '${version}' exists at:"
    echo "  https://github.com/${REPO}/releases"
    rm -rf "$tmp_dir"
    exit 1
  fi

  # extract
  tar -xzf "${tmp_dir}/${archive_name}" -C "$tmp_dir"

  # install binary
  mkdir -p "$bin_dir"
  if [[ -f "${tmp_dir}/mv" ]]; then
    install -m 0755 "${tmp_dir}/mv" "${bin_dir}/mv"
  elif [[ -f "${tmp_dir}/mindvault-${version}/mv" ]]; then
    install -m 0755 "${tmp_dir}/mindvault-${version}/mv" "${bin_dir}/mv"
  else
    echo "Error: 'mv' binary not found in archive" >&2
    rm -rf "$tmp_dir"
    exit 1
  fi

  rm -rf "$tmp_dir"

  echo ""
  echo "MindVault ${version} installed to ${bin_dir}/mv"

  # PATH hint
  if [[ -z "$NO_MODIFY_PATH" ]] && ! echo "$PATH" | tr ':' '\n' | grep -qx "$bin_dir"; then
    echo ""
    echo "Add ${bin_dir} to your PATH:"
    echo "  export PATH=\"${bin_dir}:\$PATH\""
  fi

  echo ""
  echo "Get started:"
  echo "  mv --help"
  echo "  mv server start --foreground"
}

main
