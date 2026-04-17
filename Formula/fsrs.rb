class Fsrs < Formula
  desc "CLI tool for FSRS (Free Spaced Repetition Scheduler)"
  homepage "https://github.com/open-spaced-repetition/fsrs-cli"
  version "0.1.0"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/open-spaced-repetition/fsrs-cli/releases/download/v#{version}/fsrs-aarch64-apple-darwin.tar.gz"
      # sha256 will be filled in by the release workflow
      sha256 "PLACEHOLDER"
    end

    on_intel do
      url "https://github.com/open-spaced-repetition/fsrs-cli/releases/download/v#{version}/fsrs-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/open-spaced-repetition/fsrs-cli/releases/download/v#{version}/fsrs-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end

    on_intel do
      url "https://github.com/open-spaced-repetition/fsrs-cli/releases/download/v#{version}/fsrs-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "fsrs"
  end

  test do
    assert_match "fsrs", shell_output("#{bin}/fsrs --version")
  end
end
