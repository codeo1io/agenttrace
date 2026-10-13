#!/usr/bin/env bash
set -euo pipefail

# rm-1037 pack gate (run 4aaa4c9fd914 cycle 2, batch "release-artifact
# truth"). npm/package.json's files list named a "lib" directory that does
# not exist in the tree: lib/ is created by scripts/install.js at
# postinstall time on the END-USER's machine and is never packed (npm
# silently skips files entries that match nothing — `npm pack --dry-run`
# packed 6 files, none from lib). The entry read as a shipped artifact
# while packing nothing. This gate fails when any files entry matches
# nothing under npm/, so a future add-or-remove of lib/ (or any other
# path) must be a conscious change, not silent drift. The npm package's
# channel shape itself is owned by the rm-930 release-channel identity
# decision; this gate only enforces pack-time truth whatever that
# decision lands as.

fail() {
	echo "check-npm-files: $*" >&2
	exit 1
}

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
manifest="$repo_root/npm/package.json"
pkgdir="$repo_root/npm"

[ -f "$manifest" ] || fail "npm/package.json not found at $manifest"

command -v node >/dev/null 2>&1 || fail "node is required to read npm/package.json"

if ! node - "$manifest" "$pkgdir" <<'NODE'
	const [manifestPath, pkgDir] = process.argv.slice(2);
	const fs = require("node:fs");
	const path = require("node:path");

	const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
	const files = Array.isArray(manifest.files) ? manifest.files : [];
	if (files.length === 0) {
		console.log("check-npm-files: no files entries — nothing to verify");
		process.exit(0);
	}

	// Recursive listing of everything under npm/ (files AND directories —
	// npm packs a directory entry as its whole subtree, so a directory
	// that exists counts as matching even when empty).
	const listing = [];
	(function walk(dir, rel) {
		for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
			if (ent.name === "node_modules" || ent.name === ".git") continue;
			const r = rel ? rel + "/" + ent.name : ent.name;
			listing.push(r);
			if (ent.isDirectory()) walk(path.join(dir, ent.name), r);
		}
	})(pkgDir, "");

	const problems = [];
	for (const entry of files) {
		if (typeof entry !== "string" || entry === "") {
			problems.push(JSON.stringify(entry) + " (not a non-empty string)");
			continue;
		}
		// `!` exclusions only remove paths other entries include; they do
		// not need to match anything on their own.
		if (entry.startsWith("!")) continue;
		const pattern = entry
			.replace(/[.+^${}()|[\]\\]/g, "\\$&")
			.replace(/\*\*/g, "\u0000")
			.replace(/\*/g, "[^/]*")
			.replace(/\?/g, ".")
			.replace(/\u0000/g, ".*");
		const re = new RegExp("^" + pattern + "/?$");
		if (!listing.some((p) => re.test(p))) {
			problems.push('"' + entry + '" matches nothing under npm/');
		}
	}
	if (problems.length > 0) {
		console.error(
			"files entries that match nothing in the tree (npm pack drops them silently):",
		);
		for (const p of problems) console.error("  " + p);
		process.exit(1);
	}
	console.log(
		"check-npm-files: all " + files.length + " files entries match the npm/ tree (pack truth)",
	);
NODE
then
	fail "every npm/package.json files entry must match something under npm/"
fi
