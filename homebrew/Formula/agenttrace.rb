class Agenttrace < Formula
  desc "TUI observability for AI coding agent sessions, cost, latency, and anomalies"
  # Both channels serve the codeo1io fork: stable pins its latest release
  # (v0.10.1 source tarball, sha256 re-verified against codeload on
  # 2026-10-22; .sha256 sidecars ship on the release page), head builds
  # the fork's master. Neither channel may build the upstream repo.
  homepage "https://github.com/codeo1io/agenttrace"
  url "https://github.com/codeo1io/agenttrace/archive/refs/tags/v0.10.1.tar.gz"
  sha256 "d51fbc39c7bdbebc1706707a92059eecad726eb09b362a4c7762ed78300b8a5b"
  head "https://github.com/codeo1io/agenttrace.git", branch: "master"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/agenttrace-cli")
  end

  test do
    assert_match "agenttrace v", shell_output("#{bin}/agenttrace --version")
  end
end
