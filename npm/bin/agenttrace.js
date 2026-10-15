#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const { existsSync } = require("node:fs");
const { join } = require("node:path");

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
	process.kill(process.pid, result.signal);
	// process.kill is synchronous for self-signals with default handlers; the
	// exit below only runs if the handler was overridden and did not exit.
	process.exit(128 + signalNumber(result.signal));
}
process.exit(1);

function signalNumber(signal) {
	const numbers = {
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
	return numbers[signal] ?? 1;
}
