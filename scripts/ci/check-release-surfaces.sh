#!/usr/bin/env bash
set -euo pipefail

fail() {
	echo "check-release-surfaces: $*" >&2
	exit 1
}

workspace_version="$(sed -nE 's/^version = "([^"]+)"/\1/p' Cargo.toml | head -1)"
[[ "$workspace_version" = "0.0.0-dev" ]] ||
	fail "workspace must use the development-version placeholder"

grep -q "Homebrew-tap-" README.md ||
	fail "README must link to the Homebrew tap without claiming an unpublished version"
grep -q "Homebrew-tap-" README.zh-CN.md ||
	fail "README.zh-CN must link to the Homebrew tap without claiming an unpublished version"
grep -q "AGENTTRACE_RELEASE_VERSION" .github/workflows/release.yml ||
	fail "release workflow must derive the binary version from the release tag"
grep -q "manifest.version = process.env.RELEASE_VERSION" .github/workflows/release.yml ||
	fail "release workflow must derive the npm package version from the release tag"
grep -q "cargo build --release -p agenttrace" install.sh ||
	fail "install.sh source-build fallback must use cargo"
grep -q "cargo build --release -p agenttrace" install.ps1 ||
	fail "install.ps1 source-build fallback must use cargo"
grep -q -- '--version' install.sh ||
	fail "install.sh must verify the downloaded binary runs before installing it"
grep -q "AGENTTRACE_SKIP_SOURCE_BUILD" install.sh ||
	fail "install.sh must expose a source-build skip knob for offline verification"
grep -q "Falling back to building from source" install.sh ||
	fail "install.sh must fall back to a cargo source build when the prebuilt binary is incompatible"
grep -q -- '--version' install.ps1 ||
	fail "install.ps1 must verify the installed binary runs"
grep -q "verifyRuntime" npm/scripts/install.js ||
	fail "npm installer must verify the installed binary runs"
grep -q "cargo install" README.md ||
	fail "README install paths must include cargo install"
grep -q "cargo install" README.zh-CN.md ||
	fail "README.zh-CN install paths must include cargo install"
grep -q "brew install luoyuctl/tap/agenttrace" README.md ||
	fail "README must document Homebrew installation"
# WinGet is no longer a release channel (adopted upstream v0.10.1 #301/#312
# channel decision): install.ps1 / install.cmd serve Windows instead.
if grep -R -qi "winget" .github/workflows scripts/release README.md README.zh-CN.md; then
	fail "WinGet is no longer a release channel; use install.ps1 on Windows"
fi
grep -q "install.ps1" install.cmd ||
	fail "install.cmd must delegate to install.ps1"
grep -q "npm install -g @zack78/agenttrace" README.md ||
	fail "README must document npm installation"
grep -q "npm install -g @zack78/agenttrace" README.zh-CN.md ||
	fail "README.zh-CN must document npm installation"
grep -q '"name": "@zack78/agenttrace"' npm/package.json ||
	fail "npm package must use the @zack78/agenttrace name"
grep -q '"postinstall": "node scripts/install.js"' npm/package.json ||
	fail "npm package must install the matching native release binary"
grep -q '"access": "public"' npm/package.json ||
	fail "npm package must declare public publish access"
grep -q "checksum mismatch" npm/scripts/install.js ||
	fail "npm installer must verify release checksums"
grep -q "render-channels.sh" .github/workflows/release.yml ||
	fail "release workflow must render package-channel artifacts"
grep -q "Publish npm launcher" .github/workflows/release.yml ||
	fail "release workflow must publish the npm launcher"
grep -q "Publish Homebrew Formula" .github/workflows/release.yml ||
	fail "release workflow must publish the Homebrew Formula"

for target in \
	x86_64-unknown-linux-gnu \
	aarch64-unknown-linux-gnu \
	x86_64-apple-darwin \
	aarch64-apple-darwin \
	x86_64-pc-windows-msvc \
	aarch64-pc-windows-msvc; do
	grep -q "target: $target" .github/workflows/release.yml ||
		fail "release workflow missing Rust target $target"
done

for asset in \
	agenttrace-linux-amd64 \
	agenttrace-linux-arm64 \
	agenttrace-darwin-amd64 \
	agenttrace-darwin-arm64 \
	agenttrace-windows-amd64.exe \
	agenttrace-windows-arm64.exe; do
	grep -q "asset: $asset" .github/workflows/release.yml ||
		fail "release workflow missing asset $asset"
done

grep -q "cargo build --release -p agenttrace --target" .github/workflows/release.yml ||
	fail "release workflow must build Rust release binaries"
grep -q "dtolnay/rust-toolchain" .github/workflows/release.yml ||
	fail "release workflow must set up Rust"
grep -q "gh release create" .github/workflows/release.yml ||
	fail "release workflow must publish GitHub releases"

if grep -R -Eqi "go install github.com/luoyuctl/agenttrace|go build .*cmd/agenttrace|setup-go|go-version-file" \
	README.md README.zh-CN.md CONTRIBUTING.md docs/maintainers/launch-kit.md docs/maintainers/demo-playbook.md docs/guides/parser-guide.md docs/maintainers/agentops-prompt-rules.md install.sh install.ps1 .github homebrew skills; then
	fail "public release surfaces must not advertise Go build/install paths"
fi

if grep -R -Eqi "pkg.go.dev|goreportcard|img.shields.io/badge/go-" README.md README.zh-CN.md docs/maintainers/launch-kit.md; then
	fail "README badges must advertise the Rust default implementation, not Go"
fi

if grep -R -Eqi "built in Go|Bubble Tea terminal UI" README.md README.zh-CN.md .codex-plugin skills; then
	fail "public surfaces must describe the Rust ratatui implementation"
fi

for asset in assets/readme-real-overview.png assets/readme-real-diagnostics.png; do
	[[ -f "$asset" ]] || fail "plugin screenshot asset is missing: $asset"
	grep -q "\"./$asset\"" .codex-plugin/plugin.json ||
		fail "plugin manifest must reference existing screenshot $asset"
done

if grep -R -Eqi "setup-go|(^|[^[:alnum:]_/-])go[[:space:]]+build|GOOS|GOARCH" .github/workflows; then
	fail "GitHub workflows must not use Go release/build matrix settings"
fi

if [[ -e go.mod || -e go.sum || -d cmd || -d internal ]]; then
	fail "Go implementation files must not be present in the Rust-only tree"
fi

if git ls-files '*.go' go.mod go.sum 'cmd/**' 'internal/**' | while IFS= read -r path; do [[ -e "$path" ]] && printf '%s\n' "$path"; done | grep -q .; then
	fail "Go implementation files must not be tracked in the Rust-only tree"
fi

npm_version="$(node -p "require('./npm/package.json').version")"
[[ "$npm_version" = "0.0.0-release" ]] ||
	fail "npm package must keep the release-version placeholder outside release CI"

# rm-1000 (run cebf20d5 cycle 2, batch "trusted instruments"):
# example/ref divergence arm. install.sh's pinned REF is the release
# the installer actually downloads; the README install examples pin
# the same release by name. The two had drifted (README showed
# v0.9.1 while install.sh shipped v0.10.1), and nothing failed until
# a user copied the example onto a tag the install script no longer
# fetched. Pin them together: every version literal on the README
# AGENTTRACE_VERSION lines (both languages, env and PowerShell forms)
# must equal the installer's default REF.
install_ref="$(sed -nE 's/^REF="\$\{AGENTTRACE_SOURCE_REF:-(v[0-9]+\.[0-9]+\.[0-9]+)\}"$/\1/p' install.sh | head -1)"
[[ -n "$install_ref" ]] ||
	fail "could not extract the pinned default ref from install.sh (REF= pattern moved?)"
for readme in README.md README.zh-CN.md; do
	while IFS= read -r pinned; do
		[[ "$pinned" == "$install_ref" ]] ||
			fail "$readme pins '$pinned' but install.sh defaults to '$install_ref' — the install example must match the shipped ref"
	done < <(grep -oE '(AGENTTRACE_VERSION=|-Version )v[0-9]+\.[0-9]+\.[0-9]+' "$readme" | grep -oE 'v[0-9]+\.[0-9]+\.[0-9]+' | sort -u)
done
