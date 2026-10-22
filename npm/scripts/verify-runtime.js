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

// rm-340 rider (run 212f7e87 cycle 1, batch "gemini-cli v0.65
// resume-dedupe truth + lane-parity hygiene"): the probe used
// spawnSync with no timeout, so a downloaded binary that hung on
// `--version` (corrupt-but-executable asset, or a hostile stub that
// never exits) wedged the installer forever. The probe now always
// runs under a wall-clock bound — `--version` is a print-and-exit
// call, so 30 s is generous — and a kill on the bound reports a
// not-runnable verdict with the reason in stderrTail instead of a
// hang. Tests override the bound through the second argument.
const DEFAULT_VERIFY_TIMEOUT_MS = 30_000;

function verifyRuntime(
	binaryPath,
	{ timeoutMs = DEFAULT_VERIFY_TIMEOUT_MS } = {},
) {
	const result = spawnSync(binaryPath, ["--version"], {
		encoding: "utf8",
		timeout: timeoutMs,
	});
	if (result.error) {
		if (result.error.code === "ETIMEDOUT") {
			return {
				ok: false,
				stderrTail: `agenttrace --version timed out after ${timeoutMs} ms (binary does not exit on this host)`,
			};
		}
		return { ok: false, stderrTail: String(result.error) };
	}
	if (result.signal !== null && result.status === null) {
		// Node versions that surface the timeout as a kill signal
		// without an ETIMEDOUT error object land here — as does any
		// binary that dies to a signal before printing a version.
		return {
			ok: false,
			stderrTail: `agenttrace --version was killed by ${result.signal} before exiting (binary does not run cleanly on this host)`,
		};
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
