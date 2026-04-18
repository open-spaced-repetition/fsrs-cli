#!/bin/sh
set -eu

REPO="open-spaced-repetition/fsrs-cli"
BIN_NAME="fsrs"

usage() {
  cat <<'EOF'
Usage: install.sh [--update] [--version <version>] [--dir <install-dir>] [--help]

Options:
  --update             Update an existing fsrs installation
  --version <version>  Install a specific version, e.g. v0.1.0 or 0.1.0
  --dir <install-dir>  Install directory. Defaults to $INSTALL_DIR or ~/.local/bin
  --help               Show this help

Environment:
  INSTALL_DIR          Override install directory
EOF
}

log() {
  printf '%s\n' "$*"
}

fail() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

detect_os() {
  os_name=$(uname -s 2>/dev/null || true)
  case "$os_name" in
    Darwin) printf '%s\n' "apple-darwin" ;;
    Linux) printf '%s\n' "unknown-linux-gnu" ;;
    *)
      fail "unsupported operating system: ${os_name:-unknown}"
      ;;
  esac
}

detect_arch() {
  arch_name=$(uname -m 2>/dev/null || true)
  case "$arch_name" in
    x86_64|amd64) printf '%s\n' "x86_64" ;;
    arm64|aarch64) printf '%s\n' "aarch64" ;;
    *)
      fail "unsupported architecture: ${arch_name:-unknown}"
      ;;
  esac
}

compute_sha() {
  file_path="$1"

  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file_path" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file_path" | awk '{print $1}'
  else
    fail "missing required command: shasum or sha256sum"
  fi
}

normalize_version() {
  version="$1"
  case "$version" in
    "") printf '%s\n' "latest" ;;
    v*) printf '%s\n' "$version" ;;
    *) printf 'v%s\n' "$version" ;;
  esac
}

current_version() {
  binary_path="$1"
  "$binary_path" --version 2>/dev/null | awk 'NR == 1 {print $NF}'
}

resolve_latest_version() {
  release_json="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest")" || return 1
  printf '%s' "$release_json" | tr -d '\n' | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p'
}

VERSION="${FSRS_VERSION:-}"
INSTALL_DIR="${INSTALL_DIR:-}"
UPDATE=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --update)
      UPDATE=1
      shift
      ;;
    --version)
      [ "$#" -ge 2 ] || fail "--version requires a value"
      VERSION="$2"
      shift 2
      ;;
    --dir)
      [ "$#" -ge 2 ] || fail "--dir requires a value"
      INSTALL_DIR="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

need_cmd curl
need_cmd tar
need_cmd mktemp

OS="$(detect_os)"
ARCH="$(detect_arch)"
TARGET="${ARCH}-${OS}"
ASSET_NAME="${BIN_NAME}-${TARGET}.tar.gz"
VERSION_TAG="$(normalize_version "$VERSION")"
CURRENT_PATH="$(command -v "$BIN_NAME" 2>/dev/null || true)"

if [ "$UPDATE" -eq 1 ] && [ -z "$INSTALL_DIR" ]; then
  [ -n "$CURRENT_PATH" ] || fail "${BIN_NAME} is not installed or not in PATH; use without --update or pass --dir"
  INSTALL_DIR="$(dirname "$CURRENT_PATH")"
fi

if [ -z "$INSTALL_DIR" ]; then
  INSTALL_DIR="$HOME/.local/bin"
fi

if [ "$VERSION_TAG" = "latest" ]; then
  ASSET_URL="https://github.com/${REPO}/releases/latest/download/${ASSET_NAME}"
  CHECKSUM_URL="https://github.com/${REPO}/releases/latest/download/checksums.txt"
else
  ASSET_URL="https://github.com/${REPO}/releases/download/${VERSION_TAG}/${ASSET_NAME}"
  CHECKSUM_URL="https://github.com/${REPO}/releases/download/${VERSION_TAG}/checksums.txt"
fi

TARGET_VERSION="$VERSION_TAG"
if [ "$VERSION_TAG" = "latest" ]; then
  TARGET_VERSION="$(resolve_latest_version)"
  [ -n "$TARGET_VERSION" ] || fail "failed to resolve latest release version"
fi

CURRENT_PATH_IN_DIR="${INSTALL_DIR}/${BIN_NAME}"
CURRENT_VERSION=""
if [ -x "$CURRENT_PATH_IN_DIR" ]; then
  CURRENT_VERSION="$(current_version "$CURRENT_PATH_IN_DIR" || true)"
elif [ -n "$CURRENT_PATH" ]; then
  CURRENT_VERSION="$(current_version "$CURRENT_PATH" || true)"
fi

if [ "$UPDATE" -eq 1 ] && [ -n "$CURRENT_VERSION" ] && [ "$CURRENT_VERSION" = "${TARGET_VERSION#v}" ]; then
  log "${BIN_NAME} is already up to date (${CURRENT_VERSION})"
  exit 0
fi

TMP_DIR="$(mktemp -d)"
ARCHIVE_PATH="${TMP_DIR}/${ASSET_NAME}"
CHECKSUM_PATH="${TMP_DIR}/checksums.txt"
trap 'rm -rf "$TMP_DIR"' 0 HUP INT TERM

log "Downloading ${ASSET_NAME}..."
curl -fsSL "$ASSET_URL" -o "$ARCHIVE_PATH" || fail "failed to download ${ASSET_URL}"
curl -fsSL "$CHECKSUM_URL" -o "$CHECKSUM_PATH" || fail "failed to download ${CHECKSUM_URL}"

EXPECTED_SUM="$(
  awk -v asset="$ASSET_NAME" '
    $NF == asset || $NF ~ ("/" asset "$") {
      print $1
      exit
    }
  ' "$CHECKSUM_PATH"
)"
[ -n "$EXPECTED_SUM" ] || fail "checksum for ${ASSET_NAME} not found"

ACTUAL_SUM="$(compute_sha "$ARCHIVE_PATH")"
[ "$EXPECTED_SUM" = "$ACTUAL_SUM" ] || fail "checksum mismatch for ${ASSET_NAME}"

mkdir -p "$INSTALL_DIR"
tar -xzf "$ARCHIVE_PATH" -C "$TMP_DIR"
install_path="${INSTALL_DIR}/${BIN_NAME}"
cp "${TMP_DIR}/${BIN_NAME}" "$install_path"
chmod 755 "$install_path"

if [ "$UPDATE" -eq 1 ]; then
  if [ -n "$CURRENT_VERSION" ]; then
    log "Updated ${BIN_NAME} from ${CURRENT_VERSION} to ${TARGET_VERSION#v} at ${install_path}"
  else
    log "Installed ${BIN_NAME} ${TARGET_VERSION#v} to ${install_path}"
  fi
else
  log "Installed ${BIN_NAME} ${TARGET_VERSION#v} to ${install_path}"
fi
case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    log "Warning: ${INSTALL_DIR} is not in PATH"
    log "Add it with: export PATH=\"${INSTALL_DIR}:\$PATH\""
    ;;
esac

"$install_path" --version
