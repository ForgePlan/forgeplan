class Forgeplan < Formula
  desc "Forge your plan — structured artifacts with quality scoring"
  homepage "https://github.com/ForgePlan/forgeplan"
  license "MIT"
  version "1.0.0"

  livecheck do
    url :stable
    regex(/^v?(\d+(?:\.\d+)+)$/i)
  end

  on_macos do
    on_arm do
      url "https://github.com/ForgePlan/forgeplan/releases/download/v#{version}/forgeplan-aarch64-apple-darwin"
      sha256 "PLACEHOLDER_SHA256_ARM64"
    end
    on_intel do
      url "https://github.com/ForgePlan/forgeplan/releases/download/v#{version}/forgeplan-x86_64-apple-darwin"
      sha256 "PLACEHOLDER_SHA256_X86_64"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/ForgePlan/forgeplan/releases/download/v#{version}/forgeplan-x86_64-unknown-linux-gnu"
      sha256 "PLACEHOLDER_SHA256_LINUX"
    end
  end

  def install
    binary_name = stable.url.split("/").last
    bin.install binary_name => "forgeplan"
  end

  test do
    assert_match "forgeplan #{version}", shell_output("#{bin}/forgeplan --version")
  end
end
