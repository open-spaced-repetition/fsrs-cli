class Fsrs < Formula
  desc "CLI tool for FSRS (Free Spaced Repetition Scheduler)"
  homepage "https://github.com/open-spaced-repetition/fsrs-cli"
  version "0.1.0"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/open-spaced-repetition/fsrs-cli/releases/download/v#{version}/fsrs-aarch64-apple-darwin.tar.gz"
      sha256 "1c42a6b2300187a0e0cffc82b8a9de556931dc4de1d228919736f07ccb814ec5"
    end

    on_intel do
      url "https://github.com/open-spaced-repetition/fsrs-cli/releases/download/v#{version}/fsrs-x86_64-apple-darwin.tar.gz"
      sha256 "fa7f082b551d958ffed4a4fdc264e406ff8db2d84b5c68c8076a38676d0270bb"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/open-spaced-repetition/fsrs-cli/releases/download/v#{version}/fsrs-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "07bced34f438bacab2915c1d32d4fdb8bceaab64f3e2023c958f94652e56c53c"
    end

    on_intel do
      url "https://github.com/open-spaced-repetition/fsrs-cli/releases/download/v#{version}/fsrs-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "6ff27d6c530f0edcf32580ed6e524ad802345938573dcd986917ae76fbdd5d6c"
    end
  end

  def install
    bin.install "fsrs"
  end

  test do
    assert_match "fsrs", shell_output("#{bin}/fsrs --version")
  end
end
