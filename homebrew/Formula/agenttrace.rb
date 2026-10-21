class Agenttrace < Formula
  desc "TUI observability for AI coding agent sessions, cost, latency, and anomalies"
  homepage "https://github.com/luoyuctl/agenttrace"
  head "https://github.com/luoyuctl/agenttrace.git", branch: "master"
  license "MIT"
  # rm-2243: this formula used to carry ONLY the HEAD stanza, so a
  # repo-local `brew install --formula ./homebrew/Formula/agenttrace.rb`
  # silently built the (frozen) upstream master source instead of any
  # released binary. The stable arm below pins the published release
  # artifacts — the same shape scripts/release/render-channels.sh
  # pushes to luoyuctl/homebrew-tap at release time — and
  # scripts/ci/check-release-surfaces.sh fails if it ever regresses to
  # HEAD-only, loses a platform sha256, or drifts from the installer's
  # release default.
  version "0.10.1"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/luoyuctl/agenttrace/releases/download/v0.10.1/agenttrace-darwin-arm64"
      sha256 "6f4657beb6dfe6978e5c13351ff9e31fbc7e174e00ee961949864b2d3832da85"
    else
      url "https://github.com/luoyuctl/agenttrace/releases/download/v0.10.1/agenttrace-darwin-amd64"
      sha256 "fbee6c51aa0f022ad1cfc430fc9259a589e82d9505351bd1a22ee5971102b207"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/luoyuctl/agenttrace/releases/download/v0.10.1/agenttrace-linux-arm64"
      sha256 "e111e5b0a0bde25572b5f2cf335aca1b4bceffc845acdca1f638186a24b6cc41"
    else
      url "https://github.com/luoyuctl/agenttrace/releases/download/v0.10.1/agenttrace-linux-amd64"
      sha256 "3f0159f2eca56717cc5e76a317e495b464fbdc5c9f2610665ef574c6a6d311b6"
    end
  end

  depends_on "rust" => :build if build.head?

  def install
    if build.head?
      system "cargo", "install", *std_cargo_args(path: "crates/agenttrace-cli")
    else
      bin.install Dir["agenttrace-*"].first => "agenttrace"
      chmod 0755, bin/"agenttrace"
    end
  end

  test do
    assert_match "agenttrace v", shell_output("#{bin}/agenttrace --version")
  end
end
