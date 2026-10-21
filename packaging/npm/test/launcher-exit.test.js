// rm-1438: the npm launcher must preserve the binary's exit contract.
//
// Numeric exit codes pass through unchanged (regression pin), and signal
// deaths (SIGINT/SIGTERM) are re-raised by the wrapper instead of being
// collapsed to a generic 1 — `spawnSync` reports a signal death as
// `status: null` + `signal: "SIGINT"`, which the old `status ?? 1`
// coerced to rc 1, hiding termination semantics from shells and CI.

const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const { mkdirSync, writeFileSync, chmodSync, copyFileSync, rmSync } = require("node:fs");
const { tmpdir } = require("node:os");
const { join } = require("node:path");
const test = require("node:test");

const packageRoot = join(__dirname, "..");
const launcher = join(packageRoot, "bin", "agenttrace.js");

function scaffold() {
	const root = join(tmpdir(), `at-launcher-test-${process.pid}-${Date.now()}`);
	mkdirSync(join(root, "lib"), { recursive: true });
	mkdirSync(join(root, "bin"), { recursive: true });
	// The wrapper resolves its binary relative to its own location
	// (`join(__dirname, "..", "lib", ...)`), so the scaffold mirrors that
	// layout with the real launcher and a controllable stub binary.
	copyFileSync(launcher, join(root, "bin", "agenttrace.js"));
	return root;
}

function stubBinary(root, script) {
	const bin = join(root, "lib", "agenttrace");
	writeFileSync(bin, script, "utf8");
	chmodSync(bin, 0o755);
	return bin;
}

function runLauncher(root, args) {
	return spawnSync(process.execPath, [join(root, "bin", "agenttrace.js"), ...args], {
		encoding: "utf8",
	});
}

test("numeric exit codes pass through the wrapper unchanged", () => {
	const root = scaffold();
	try {
	stubBinary(
		root,
		process.platform === "win32"
			? "@echo off\r\nexit /b 42\r\n"
			: "#!/bin/sh\nexit 42\n",
	);
	const result = runLauncher(root, ["--version"]);
	assert.equal(result.status, 42, `rc must pass through, got ${result.status}`);
	assert.equal(result.signal, null);
	} finally {
		rmSync(root, { recursive: true, force: true });
	}
});

test("success exit code zero passes through", () => {
	const root = scaffold();
	try {
	stubBinary(root, process.platform === "win32" ? "@echo off\r\nexit /b 0\r\n" : "#!/bin/sh\nexit 0\n");
	const result = runLauncher(root, ["--overview"]);
	assert.equal(result.status, 0);
	assert.equal(result.signal, null);
	} finally {
		rmSync(root, { recursive: true, force: true });
	}
});

test("SIGINT death is re-raised, not converted to exit 1", () => {
	if (process.platform === "win32") {
		return; // signals are not POSIX-real on Windows; numeric arm covers it
	}
	const root = scaffold();
	try {
	stubBinary(root, "#!/bin/sh\nkill -INT $$\n");
	const result = runLauncher(root, ["--overview"]);
	assert.equal(result.status, null, "wrapper must die by the same signal, not exit");
	assert.equal(result.signal, "SIGINT", "SIGINT must be preserved across the boundary");
	} finally {
		rmSync(root, { recursive: true, force: true });
	}
});

test("SIGTERM death is re-raised, not converted to exit 1", () => {
	if (process.platform === "win32") {
		return;
	}
	const root = scaffold();
	try {
	stubBinary(root, "#!/bin/sh\nkill -TERM $$\n");
	const result = runLauncher(root, ["--overview"]);
	assert.equal(result.status, null);
	assert.equal(result.signal, "SIGTERM", "SIGTERM must be preserved across the boundary");
	} finally {
		rmSync(root, { recursive: true, force: true });
	}
});

// rm-1873: Node documents (process.kill) that win32 emulates only
// SIGINT/SIGTERM/SIGKILL — every other signal in the launcher's table
// must clamp to the 128+signalNumber exit fallback instead of reaching
// process.kill, which would THROW on a platform package.json declares
// supported. Mapping-table tests run on ALL platforms (no spawn, no
// platform guards) — they pin the clamp decision itself.
const launcherModule = require(launcher);
const SIGNAL_TABLE = Object.keys(launcherModule.SIGNAL_NUMBERS);
const WIN32_EMULATED = new Set(["SIGINT", "SIGTERM", "SIGKILL"]);

test("win32 clamps every non-emulated signal to the numeric fallback", () => {
	assert.ok(SIGNAL_TABLE.length >= 15, `signal table must be covered, got ${SIGNAL_TABLE.length}`);
	for (const signal of SIGNAL_TABLE) {
		assert.equal(
			launcherModule.reRaiseableOn("win32", signal),
			WIN32_EMULATED.has(signal),
			`win32 re-raise decision for ${signal} must follow the Node emulation contract`,
		);
	}
	// Unknown signals (not in the table) clamp too — never reach process.kill.
	assert.equal(launcherModule.reRaiseableOn("win32", "SIGWRAP"), false);
});

test("posix platforms re-raise every signal in the table", () => {
	for (const platform of ["linux", "darwin", "freebsd"]) {
		for (const signal of SIGNAL_TABLE) {
			assert.equal(
				launcherModule.reRaiseableOn(platform, signal),
				true,
				`${platform} must re-raise ${signal}`,
			);
		}
	}
});
