#!/usr/bin/env bash
set -euo pipefail

ARTIFACTS_DIR="${1:-artifacts}"

sha_for_tarball() {
  local target="$1"
  local artifact_path="${ARTIFACTS_DIR}/fsrs-${target}/fsrs-${target}.tar.gz"

  sha256sum "$artifact_path" | cut -d' ' -f1
}

{
  echo "VERSION=${GITHUB_REF_NAME#v}"
  echo "SHA_MACOS_ARM=$(sha_for_tarball aarch64-apple-darwin)"
  echo "SHA_MACOS_INTEL=$(sha_for_tarball x86_64-apple-darwin)"
  echo "SHA_LINUX_ARM=$(sha_for_tarball aarch64-unknown-linux-gnu)"
  echo "SHA_LINUX_INTEL=$(sha_for_tarball x86_64-unknown-linux-gnu)"
} >> "$GITHUB_ENV"
