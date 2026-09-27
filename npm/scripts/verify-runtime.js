#!/usr/bin/env node

// Runtime verification helper for the agenttrace npm installer.
//
// A downloaded release binary can be incompatible with the host it lands on
// (for example built against a newer glibc than the host provides — Maestro
// finding 47fa1154, where the upstream linux-amd64 asset died with
// "GLIBC_2.39 not found"). install.js probes the binary with `--version`
// through this helper and refuses to leave a broken binary behind.
//
// Kept as a side-effect-free module so `node --test` can exercise it offline.

const { spawnSync } = require("node:child_process");

const STDERR_TAIL_LINES = 3;

function verifyRuntime(binaryPath) {
	const result = spawnSync(binaryPath, ["--version"], { encoding: "utf8" });
	if (result.error) {
		return { ok: false, stderrTail: String(result.error) };
	}
	if (result.status !== 0) {
		const combined = `${result.stderr || ""}\n${result.stdout || ""}`.trim();
		const stderrTail = combined
			.split("\n")
			.filter((line) => line.length > 0)
			.slice(-STDERR_TAIL_LINES)
			.join("\n");
		return { ok: false, stderrTail };
	}
	const version = `${result.stdout || ""}`.trim().split("\n")[0];
	return { ok: true, version };
}

module.exports = { verifyRuntime };
