#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const { existsSync } = require("node:fs");
const { join } = require("node:path");

const SIGNAL_NUMBERS = {
	SIGHUP: 1,
	SIGINT: 2,
	SIGQUIT: 3,
	SIGILL: 4,
	SIGTRAP: 5,
	SIGABRT: 6,
	SIGBUS: 7,
	SIGFPE: 8,
	SIGKILL: 9,
	SIGUSR1: 10,
	SIGSEGV: 11,
	SIGUSR2: 12,
	SIGPIPE: 13,
	SIGALRM: 14,
	SIGTERM: 15,
};

// rm-1873: Node documents (process.kill) that Windows does not support
// POSIX signals and emulates exactly three of them — SIGINT, SIGTERM and
// SIGKILL cause unconditional termination; anything else makes
// process.kill(pid, signal) THROW on win32. The launcher's exit contract
// must clamp non-emulated signals to the numeric fallback on win32
// instead of re-raising them.
const WIN32_KILL_EMULATED = new Set(["SIGINT", "SIGTERM", "SIGKILL"]);

function signalNumber(signal) {
	return SIGNAL_NUMBERS[signal] ?? 1;
}

function reRaiseableOn(platform, signal) {
	return platform !== "win32" || WIN32_KILL_EMULATED.has(signal);
}

function main() {
	const executable = join(
		__dirname,
		"..",
		"lib",
		process.platform === "win32" ? "agenttrace.exe" : "agenttrace",
	);

	if (!existsSync(executable)) {
		console.error(
			"agenttrace could not find its platform binary. Reinstall it with `npm install -g @zack78/agenttrace`.",
		);
		process.exit(1);
	}

	const result = spawnSync(executable, process.argv.slice(2), {
		stdio: "inherit",
	});

	if (result.error) {
		console.error(`agenttrace failed to start: ${result.error.message}`);
		process.exit(1);
	}

	// rm-1438: preserve the binary's exit contract at the npm boundary.
	// `spawnSync` reports a signal death as `status: null` plus the signal name,
	// so the old `status ?? 1` collapsed SIGINT/SIGTERM exits to a generic 1.
	// Numeric statuses pass through unchanged; signal deaths are re-raised so
	// shells and CI observe the same termination semantics as the bare binary.
	if (result.status !== null && result.status !== undefined) {
		process.exit(result.status);
	}
	if (result.signal) {
		// rm-1873: only SIGINT/SIGTERM/SIGKILL are emulated on win32 — a
		// re-raise of anything else would throw and break the exit contract
		// on a platform package.json declares supported, so those clamp to
		// the 128+signalNumber fallback below.
		if (reRaiseableOn(process.platform, result.signal)) {
			process.kill(process.pid, result.signal);
			// process.kill is synchronous for self-signals with default
			// handlers; the exit below only runs if the handler was
			// overridden and did not exit.
		}
		process.exit(128 + signalNumber(result.signal));
	}
	process.exit(1);
}

module.exports = { SIGNAL_NUMBERS, signalNumber, reRaiseableOn };

if (require.main === module) {
	main();
}
