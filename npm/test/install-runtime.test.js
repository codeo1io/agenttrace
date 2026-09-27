const assert = require("node:assert/strict");
const { chmodSync, mkdtempSync, rmSync, writeFileSync } = require("node:fs");
const { tmpdir } = require("node:os");
const { join } = require("node:path");
const test = require("node:test");

const { verifyRuntime } = require(join(__dirname, "..", "scripts", "verify-runtime.js"));

function stub(dir, name, body) {
	const path = join(dir, name);
	writeFileSync(path, body);
	chmodSync(path, 0o755);
	return path;
}

test("verifyRuntime accepts a runnable binary and reports its version", () => {
	const dir = mkdtempSync(join(tmpdir(), "agenttrace-verify-ok-"));
	try {
		const good = stub(
			dir,
			"good.sh",
			'#!/bin/sh\necho "agenttrace v0.0.0-stub"\n',
		);
		const result = verifyRuntime(good);
		assert.equal(result.ok, true);
		assert.equal(result.version, "agenttrace v0.0.0-stub");
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
});

test("verifyRuntime rejects a binary that fails like an incompatible loader", () => {
	const dir = mkdtempSync(join(tmpdir(), "agenttrace-verify-bad-"));
	try {
		const bad = stub(
			dir,
			"bad.sh",
			'#!/bin/sh\necho "agenttrace: /lib64/libc.so.6: version GLIBC_2.39 not found (required by agenttrace)" >&2\nexit 1\n',
		);
		const result = verifyRuntime(bad);
		assert.equal(result.ok, false);
		assert.ok(
			result.stderrTail.includes("GLIBC_2.39"),
			`stderr tail must carry the loader error, got: ${result.stderrTail}`,
		);
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
});

test("verifyRuntime reports a missing binary as not runnable", () => {
	const missing = join(
		tmpdir(),
		`agenttrace-verify-missing-${process.pid}-no-such-file`,
	);
	const result = verifyRuntime(missing);
	assert.equal(result.ok, false);
});
