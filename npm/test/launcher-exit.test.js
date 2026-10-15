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
