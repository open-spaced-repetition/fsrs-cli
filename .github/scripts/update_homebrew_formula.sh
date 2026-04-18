#!/usr/bin/env bash
set -euo pipefail

FORMULA_PATH="${1:-Formula/fsrs.rb}"

cat > "$FORMULA_PATH" <<EOF
class Fsrs < Formula
  desc "CLI tool for FSRS (Free Spaced Repetition Scheduler)"
  homepage "https://github.com/open-spaced-repetition/fsrs-cli"
  version "${VERSION}"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/open-spaced-repetition/fsrs-cli/releases/download/v${VERSION}/fsrs-aarch64-apple-darwin.tar.gz"
      sha256 "${SHA_MACOS_ARM}"
    end

    on_intel do
      url "https://github.com/open-spaced-repetition/fsrs-cli/releases/download/v${VERSION}/fsrs-x86_64-apple-darwin.tar.gz"
      sha256 "${SHA_MACOS_INTEL}"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/open-spaced-repetition/fsrs-cli/releases/download/v${VERSION}/fsrs-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "${SHA_LINUX_ARM}"
    end

    on_intel do
      url "https://github.com/open-spaced-repetition/fsrs-cli/releases/download/v${VERSION}/fsrs-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "${SHA_LINUX_INTEL}"
    end
  end

  def install
    bin.install "fsrs"
  end

  test do
    assert_match "fsrs", shell_output("#{bin}/fsrs --version")
  end
end
EOF
