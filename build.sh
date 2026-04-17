#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

# Determine binary name based on OS
case "$(uname -s 2>/dev/null || echo Windows)" in
  MINGW*|MSYS*|CYGWIN*|Windows*)
    BIN_NAME="fsrs.exe"
    ;;
  *)
    BIN_NAME="fsrs"
    ;;
esac

TARGET="$SCRIPT_DIR/target/release/$BIN_NAME"
LINK="$SCRIPT_DIR/$BIN_NAME"

# Create symlink (Unix) or copy (Windows)
if [ -e "$TARGET" ]; then
  case "$(uname -s 2>/dev/null || echo Windows)" in
    MINGW*|MSYS*|CYGWIN*|Windows*)
      cp -f "$TARGET" "$LINK"
      ;;
    *)
      if [ ! -L "$LINK" ] || [ "$(readlink "$LINK")" != "$TARGET" ]; then
        ln -sf "$TARGET" "$LINK"
      fi
      ;;
  esac
fi

exec "$TARGET" "$@"
