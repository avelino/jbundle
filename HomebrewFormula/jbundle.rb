class Jbundle < Formula
  desc "Package JVM applications into self-contained binaries"
  homepage "https://github.com/avelino/jbundle"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/avelino/jbundle/releases/download/v#{version}/jbundle-darwin-aarch64-v#{version}.tar.gz"
      sha256 "PLACEHOLDER_DARWIN_AARCH64_SHA256"
    else
      url "https://github.com/avelino/jbundle/releases/download/v#{version}/jbundle-darwin-x86_64-v#{version}.tar.gz"
      sha256 "PLACEHOLDER_DARWIN_X86_64_SHA256"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/avelino/jbundle/releases/download/v#{version}/jbundle-linux-aarch64-v#{version}.tar.gz"
      sha256 "PLACEHOLDER_LINUX_AARCH64_SHA256"
    else
      url "https://github.com/avelino/jbundle/releases/download/v#{version}/jbundle-linux-x86_64-v#{version}.tar.gz"
      sha256 "PLACEHOLDER_LINUX_X86_64_SHA256"
    end
  end

  def install
    bin.install "jbundle"
  end

  test do
    assert_match "jbundle", shell_output("#{bin}/jbundle --version")
  end
end
