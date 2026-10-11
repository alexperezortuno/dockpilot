#!/usr/bin/env bash
set -euo pipefail

REPOSITORY="alexperezortuno/dockpilot"
VERSION="${DOCKPILOT_VERSION:-0.5.1}"
INSTALL_DIR="${DOCKPILOT_INSTALL_DIR:-$HOME/.local/bin}"

usage() {
  printf 'Usage: %s [VERSION] [--dir DIRECTORY]\n' "$0"
}

while (($#)); do
  case "$1" in
    --dir)
      (($# >= 2)) || { printf '%s\n' '--dir requires a directory' >&2; exit 2; }
      INSTALL_DIR="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    -* )
      printf 'Unknown option: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
    *)
      VERSION="$1"
      shift
      ;;
  esac
done

case "$(uname -s):$(uname -m)" in
  Linux:x86_64|Linux:amd64) PLATFORM="linux-x86_64"; EXT="tar.gz"; BINARY="dockpilot" ;;
  Darwin:arm64|Darwin:aarch64) PLATFORM="macos-aarch64"; EXT="tar.gz"; BINARY="dockpilot" ;;
  Darwin:x86_64) PLATFORM="macos-x86_64"; EXT="tar.gz"; BINARY="dockpilot" ;;
  MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64) PLATFORM="windows-x86_64"; EXT="zip"; BINARY="dockpilot.exe" ;;
  *) printf 'Unsupported platform: %s:%s\n' "$(uname -s)" "$(uname -m)" >&2; exit 1 ;;
esac

command -v curl >/dev/null 2>&1 || { printf '%s\n' 'curl is required' >&2; exit 1; }
command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 || {
  printf '%s\n' 'sha256sum or shasum is required' >&2
  exit 1
}

TAG="v${VERSION#v}"
NAME="dockpilot-${TAG}-${PLATFORM}"
BASE_URL="https://github.com/${REPOSITORY}/releases/download/${TAG}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

curl --fail --location --silent --show-error "$BASE_URL/$NAME.$EXT" -o "$TMP_DIR/$NAME.$EXT"
curl --fail --location --silent --show-error "$BASE_URL/SHA256SUMS.txt" -o "$TMP_DIR/SHA256SUMS.txt"
(
  cd "$TMP_DIR"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum --check --ignore-missing SHA256SUMS.txt
  else
    expected="$(awk -v file="$NAME.$EXT" '$2 == file {print $1}' SHA256SUMS.txt)"
    actual="$(shasum -a 256 "$NAME.$EXT" | cut -d ' ' -f 1)"
    test -n "$expected" && test "$expected" = "$actual"
  fi
)

mkdir -p "$INSTALL_DIR"
if [[ "$EXT" == tar.gz ]]; then
  tar -xzf "$TMP_DIR/$NAME.$EXT" -C "$TMP_DIR"
else
  command -v unzip >/dev/null 2>&1 || { printf '%s\n' 'unzip is required for Windows archives' >&2; exit 1; }
  unzip -q "$TMP_DIR/$NAME.$EXT" -d "$TMP_DIR"
fi
test -f "$TMP_DIR/$NAME/$BINARY"
install -m 0755 "$TMP_DIR/$NAME/$BINARY" "$INSTALL_DIR/$BINARY"
printf 'Installed %s to %s\n' "$TAG" "$INSTALL_DIR/$BINARY"
printf 'Ensure %s is on PATH, then run: dockpilot --version\n' "$INSTALL_DIR"
