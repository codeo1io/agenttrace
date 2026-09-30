#!/usr/bin/env bash
set -euo pipefail

# Behavioral check that the install surfaces verify the runtime they install.
#
# Root cause pinned by Maestro finding 47fa1154 (run 3988bfe5): the upstream
# linux-amd64 release asset was built on ubuntu-latest (glibc 2.39) and
# install.sh installed it silently on an Ubuntu 22.04 host (glibc 2.35), where
# `agenttrace --help` dies with "GLIBC_2.39 not found". install.sh must now
# prove the downloaded binary runs BEFORE installing it and fall back to a
# cargo source build otherwise.
#
# All tests run offline: curl fetches local file:// stubs (the fixtures are
# padded past install.sh's 1 MB size gate with comment lines so the runtime
# verification path is actually reached). The glibc-baseline guard at the end
# scans a locally built release binary's versioned symbol requirements; an
# upstream-v0.8.1-style binary (GLIBC_2.39) would fail it here.

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

fail() {
	echo "check-install-runtime: $*" >&2
	exit 1
}

sh -n install.sh || fail "install.sh must remain valid POSIX sh"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# — fixtures —
ok_stub="$tmp/ok-stub"
{
	echo '#!/bin/sh'
	echo 'echo "agenttrace v0.0.0-stub"'
	echo 'exit 0'
	awk 'BEGIN {
		for (i = 0; i < 40000; i++)
			print "# padding line so the stub clears the 1 MB size gate"
	}'
} >"$ok_stub"
chmod +x "$ok_stub"

bad_stub="$tmp/bad-stub"
{
	echo '#!/bin/sh'
	echo 'echo "agenttrace: /lib64/libc.so.6: version GLIBC_2.39 not found" >&2'
	echo 'exit 1'
	awk 'BEGIN {
		for (i = 0; i < 40000; i++)
			print "# padding line so the stub clears the 1 MB size gate"
	}'
} >"$bad_stub"
chmod +x "$bad_stub"

# rm-029 made a missing sidecar a refusal, so every fixture that is
# meant to reach the runtime-verification step carries its sidecar.
sha256sum "$ok_stub" | cut -d' ' -f1 >"$ok_stub.sha256"
sha256sum "$bad_stub" | cut -d' ' -f1 >"$bad_stub.sha256"

# — test A: a runnable download is installed and confirmed —
out="$tmp/out.a"
if ! AGENTTRACE_INSTALL_DIR="$tmp/a" AGENTTRACE_DOWNLOAD_URL="file://$ok_stub" \
	sh install.sh >"$out" 2>&1; then
	cat "$out" >&2
	fail "test A: install.sh must accept a runnable downloaded binary"
fi
[[ -x "$tmp/a/agenttrace" ]] || fail "test A: runnable stub must be installed"
grep -q "agenttrace v0.0.0-stub" "$out" ||
	fail "test A: post-install confirmation must print the binary's --version line"

# — test B: an incompatible download is rejected and never installed —
out="$tmp/out.b"
if AGENTTRACE_INSTALL_DIR="$tmp/b" AGENTTRACE_DOWNLOAD_URL="file://$bad_stub" \
	AGENTTRACE_SKIP_SOURCE_BUILD=1 sh install.sh >"$out" 2>&1; then
	cat "$out" >&2
	fail "test B: install.sh must reject an incompatible downloaded binary"
fi
[[ ! -e "$tmp/b/agenttrace" ]] || fail "test B: incompatible stub must not be installed"
grep -q "does not run on this host" "$out" ||
	fail "test B: rejection must say the binary does not run on this host"
grep -q "GLIBC_2.39" "$out" ||
	fail "test B: rejection must surface the binary's own error output"

# — test C: incompatible download + no toolchain → loud, actionable failure —
# A sandbox PATH holding only what install.sh legitimately needs until the
# failure (and provably no git/cargo): /usr/bin/git would otherwise leak in
# from the stock PATH on both this host and CI runners.
toolbox="$tmp/toolbox"
mkdir -p "$toolbox"
for tool in sh uname tr mkdir curl mktemp chmod wc cut head rm mv cp grep cat sha256sum; do
	tool_path="$(command -v "$tool")" ||
		fail "test C: host is missing $tool, needed to assemble the sandbox PATH"
	ln -s "$tool_path" "$toolbox/$tool"
done
# The sandbox PATH is exactly this directory, so the absence of git/cargo
# here proves the no-toolchain branch is what install.sh takes.
[[ ! -e "$toolbox/git" && ! -e "$toolbox/cargo" ]] ||
	fail "test C: sandbox PATH unexpectedly exposes git/cargo"
out="$tmp/out.c"
if env PATH="$toolbox" AGENTTRACE_INSTALL_DIR="$tmp/c" \
	AGENTTRACE_DOWNLOAD_URL="file://$bad_stub" sh install.sh >"$out" 2>&1; then
	cat "$out" >&2
	fail "test C: install.sh must fail loudly when no Rust toolchain can heal the download"
fi
[[ ! -e "$tmp/c/agenttrace" ]] || fail "test C: incompatible stub must not be installed"
grep -q "no Rust toolchain" "$out" ||
	fail "test C: failure must name the missing Rust toolchain"

# — test D: incompatible download + toolchain → cargo source-build fallback —
# The ref is pinned via AGENTTRACE_SOURCE_REF (a release tag) so the
# fallback never needs to resolve api.github.com in an offline run.
shims="$tmp/shims"
mkdir -p "$shims"
cat >"$shims/git" <<FAKE_GIT
#!/bin/sh
set -e
if [ "\$1" = "clone" ]; then
	for dest do
		:
	done
	mkdir -p "\$dest/target/release"
	cp "$ok_stub" "\$dest/target/release/agenttrace"
	exit 0
fi
exit 1
FAKE_GIT
cat >"$shims/cargo" <<'FAKE_CARGO'
#!/bin/sh
exit 0
FAKE_CARGO
chmod +x "$shims/git" "$shims/cargo"
out="$tmp/out.d"
if ! env PATH="$shims:$PATH" AGENTTRACE_INSTALL_DIR="$tmp/d" \
	AGENTTRACE_SOURCE_REF="v0.0.0-stub" \
	AGENTTRACE_DOWNLOAD_URL="file://$bad_stub" sh install.sh >"$out" 2>&1; then
	cat "$out" >&2
	fail "test D: install.sh must fall back to a source build for an incompatible download"
fi
[[ -x "$tmp/d/agenttrace" ]] || fail "test D: fallback-built stub must be installed"
cmp -s "$tmp/d/agenttrace" "$ok_stub" ||
	fail "test D: installed binary must be the fallback-built artifact"
grep -q "Falling back to building from source" "$out" ||
	fail "test D: fallback must be announced"
grep -q "Built from source successfully" "$out" ||
	fail "test D: fallback success must be announced"

# — test E: source build pinned by FULL COMMIT ID (rm-030 review F3) —
# `git clone --branch` cannot resolve a commit id (names only), so a
# SHA pin must fetch the commit directly. A real local git fixture
# (real git on PATH, only cargo shimmed) proves it end-to-end offline:
# the fetched HEAD must equal the pin, be announced as verified, and
# the built artifact must be installed.
cargoshim="$tmp/cargoshim"
mkdir -p "$cargoshim"
cat >"$cargoshim/cargo" <<'FAKE_CARGO'
#!/bin/sh
exit 0
FAKE_CARGO
chmod +x "$cargoshim/cargo"
fx="$tmp/fxrepo"
mkdir -p "$fx/target/release"
cp "$ok_stub" "$fx/target/release/agenttrace"
git -C "$fx" init -q
git -C "$fx" add -A
git -C "$fx" -c user.email=ci@example.com -c user.name=ci commit -qm fixture
FX_SHA="$(git -C "$fx" rev-parse HEAD)"
out="$tmp/out.e"
if ! env PATH="$cargoshim:$PATH" AGENTTRACE_INSTALL_DIR="$tmp/e" \
	AGENTTRACE_DOWNLOAD_URL="file://$bad_stub" \
	AGENTTRACE_SOURCE_URL="file://$fx" \
	AGENTTRACE_SOURCE_REF="$FX_SHA" sh install.sh >"$out" 2>&1; then
	cat "$out" >&2
	fail "test E: a full-commit-id pin must build and install"
fi
[[ -x "$tmp/e/agenttrace" ]] || fail "test E: SHA-pinned build must be installed"
cmp -s "$tmp/e/agenttrace" "$ok_stub" ||
	fail "test E: installed binary must be the SHA-pinned source artifact"
grep -q "Pin verified: fetched commit matches $FX_SHA" "$out" ||
	fail "test E: the fetched commit must be checked against and reported as matching the pin"

# — test F: an unresolvable commit pin refuses instead of building —
out="$tmp/out.f"
if env PATH="$cargoshim:$PATH" AGENTTRACE_INSTALL_DIR="$tmp/f" \
	AGENTTRACE_DOWNLOAD_URL="file://$bad_stub" \
	AGENTTRACE_SOURCE_URL="file://$fx" \
	AGENTTRACE_SOURCE_REF="$(printf 'd%.0s' $(seq 1 40))" \
	sh install.sh >"$out" 2>&1; then
	cat "$out" >&2
	fail "test F: an unresolvable commit pin must refuse"
fi
[[ ! -e "$tmp/f/agenttrace" ]] || fail "test F: nothing may be installed on refusal"
grep -q "Cannot fetch pinned ref" "$out" ||
	fail "test F: refusal must name the unresolvable pinned ref"

# — test G: a download with NO sidecar is refused, never installed (rm-029) —
nosha_stub="$tmp/nosha-stub"
cp "$ok_stub" "$nosha_stub"
out="$tmp/out.g"
if env AGENTTRACE_INSTALL_DIR="$tmp/g" \
	AGENTTRACE_DOWNLOAD_URL="file://$nosha_stub" sh install.sh >"$out" 2>&1; then
	cat "$out" >&2
	fail "test G: install.sh must refuse a download whose sidecar is missing"
fi
[[ ! -e "$tmp/g/agenttrace" ]] || fail "test G: nothing may be installed without a sidecar"
grep -q "No checksum sidecar at" "$out" ||
	fail "test G: refusal must diagnose the missing sidecar"

# — glibc-baseline guard: a locally built binary must not require a glibc
#   newer than the compatibility floor we defend (Ubuntu 22.04 = 2.35) —
bin="${AGENTTRACE_BIN:-}"
if [[ -z "$bin" ]]; then
	echo "glibc baseline guard: skipped (AGENTTRACE_BIN not set)"
elif [[ ! -e "$bin" ]]; then
	echo "glibc baseline guard: skipped ($bin does not exist yet; build it first)"
elif ! command -v objdump >/dev/null 2>&1; then
	echo "glibc baseline guard: skipped (objdump not available)"
else
	max_glibc="$(objdump -T "$bin" 2>/dev/null |
		grep -o 'GLIBC_[0-9]\+\(\.[0-9]\+\)*' |
		sed 's/^GLIBC_//' |
		sort -t . -k 1,1n -k 2,2n | tail -n 1 || true)"
	if [[ -z "$max_glibc" ]]; then
		echo "glibc baseline guard: no GLIBC requirements found in $bin"
	else
		echo "glibc baseline guard: $bin requires GLIBC_$max_glibc (floor: 2.35)"
		major="${max_glibc%%.*}"
		minor="${max_glibc#*.}"
		minor="${minor%%.*}"
		if ((major > 2)) || ((major == 2 && minor > 35)); then
			fail "glibc baseline guard: $bin requires GLIBC_$max_glibc > 2.35 (Ubuntu 22.04 floor)" \
				"— rebuild against an older baseline or use a static/musl target"
		fi
	fi
fi

echo "Install runtime verification passed"
