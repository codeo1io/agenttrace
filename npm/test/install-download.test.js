const assert = require("node:assert/strict");
const { EventEmitter } = require("node:events");
const { mkdtempSync, rmSync, writeFileSync } = require("node:fs");
const { tmpdir } = require("node:os");
const { join } = require("node:path");
const { PassThrough } = require("node:stream");
const test = require("node:test");

const {
	MAX_DOWNLOAD_BYTES,
	REDIRECT_ALLOWLIST,
	assertPinnedUrl,
	download,
	readPinnedChecksums,
	reconcileChecksums,
} = require(join(__dirname, "..", "scripts", "install.js"));

function response(statusCode, headers, body) {
	const stream = body === undefined ? new PassThrough() : PassThrough.from(body);
	stream.statusCode = statusCode;
	stream.headers = headers;
	if (body !== undefined) stream.end();
	return stream;
}

function stubGetter(responses) {
	let hop = 0;
	const getter = (url, callback) => {
		const next = responses[Math.min(hop, responses.length - 1)];
		hop += 1;
		callback(typeof next === "function" ? next(url) : next);
		return new EventEmitter();
	};
	return getter;
}

test("the download host allowlist pins github and its release-asset CDN", () => {
	assert.ok(REDIRECT_ALLOWLIST.has("github.com"));
	assert.ok(REDIRECT_ALLOWLIST.has("objects.githubusercontent.com"));
	assert.ok(REDIRECT_ALLOWLIST.has("release-assets.githubusercontent.com"));
});

test("assertPinnedUrl accepts allowlisted https URLs and rejects everything else", () => {
	assert.equal(
		assertPinnedUrl("https://github.com/zack78/agenttrace/releases/download/v1.0.0/x").hostname,
		"github.com",
	);
	assert.throws(() => assertPinnedUrl("https://evil.example/payload"), /not pinned/);
	assert.throws(() => assertPinnedUrl("http://github.com/payload"), /https only/);
});

test("download follows allowlisted redirect hops and writes the body", async () => {
	const dir = mkdtempSync(join(tmpdir(), "agenttrace-install-redirect-"));
	try {
		const destination = join(dir, "binary");
		await download("https://github.com/owner/repo/releases/download/v1.0.0/asset", destination, {
			getter: stubGetter([
				response(302, { location: "https://objects.githubusercontent.com/owner/repo/asset" }),
				response(200, {}, Buffer.from("release-bytes")),
			]),
		});
		const { readFileSync } = require("node:fs");
		assert.equal(readFileSync(destination, "utf8"), "release-bytes");
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
});

test("download refuses a redirect to an unpinned host and leaves no partial file", async () => {
	const dir = mkdtempSync(join(tmpdir(), "agenttrace-install-evil-"));
	try {
		const destination = join(dir, "binary");
		await assert.rejects(
			download("https://github.com/owner/repo/releases/download/v1.0.0/asset", destination, {
				getter: stubGetter([
					response(302, { location: "https://evil.example/payload" }),
				]),
			}),
			/not pinned/,
		);
		assert.equal(require("node:fs").existsSync(destination), false, "no partial file survives");
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
});

test("download refuses a non-https redirect target", async () => {
	const dir = mkdtempSync(join(tmpdir(), "agenttrace-install-downgrade-"));
	try {
		await assert.rejects(
			download("https://github.com/owner/repo/releases/download/v1.0.0/asset", join(dir, "b"), {
				getter: stubGetter([
					response(302, { location: "http://objects.githubusercontent.com/payload" }),
				]),
			}),
			/https only/,
		);
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
});

test("download enforces the byte cap and removes the partial file", async () => {
	const dir = mkdtempSync(join(tmpdir(), "agenttrace-install-cap-"));
	try {
		const destination = join(dir, "binary");
		const body = Buffer.alloc(64, 0x41);
		await assert.rejects(
			download("https://objects.githubusercontent.com/owner/repo/asset", destination, {
				maxBytes: 16,
				getter: stubGetter([response(200, {}, body)]),
			}),
			/byte cap/,
		);
		assert.equal(require("node:fs").existsSync(destination), false, "no partial file survives");
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
});

test("download rejects an oversized content-length before any body is read", async () => {
	const dir = mkdtempSync(join(tmpdir(), "agenttrace-install-length-"));
	try {
		const destination = join(dir, "binary");
		let bodyRequested = false;
		await assert.rejects(
			download("https://objects.githubusercontent.com/owner/repo/asset", destination, {
				maxBytes: 16,
				getter: stubGetter([
					() => {
						bodyRequested = true;
						return response(200, { "content-length": "4096" });
					},
				]),
			}),
			/byte cap/,
		);
		assert.equal(bodyRequested, true);
		assert.equal(require("node:fs").existsSync(destination), false, "no partial file survives");
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
});

test("the byte cap defaults to the 4 GiB acceptance arm", () => {
	assert.equal(MAX_DOWNLOAD_BYTES, 4 * 1024 * 1024 * 1024);
});

test("readPinnedChecksums returns null when absent, the map when valid, and fails loud otherwise", () => {
	const dir = mkdtempSync(join(tmpdir(), "agenttrace-install-pin-"));
	try {
		assert.equal(readPinnedChecksums(join(dir, "absent.json")), null);
		const valid = join(dir, "valid.json");
		writeFileSync(valid, JSON.stringify({ "agenttrace-linux-amd64": "a".repeat(64) }));
		assert.deepEqual(readPinnedChecksums(valid), { "agenttrace-linux-amd64": "a".repeat(64) });
		const corrupt = join(dir, "corrupt.json");
		writeFileSync(corrupt, "{nope");
		assert.throws(() => readPinnedChecksums(corrupt), /not valid JSON/);
		const badDigest = join(dir, "bad-digest.json");
		writeFileSync(badDigest, JSON.stringify({ "agenttrace-linux-amd64": "deadbeef" }));
		assert.throws(() => readPinnedChecksums(badDigest), /sha256/);
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
});

test("reconcileChecksums prefers the package pin and fails on channel disagreement", () => {
	const pin = "b".repeat(64);
	const other = "c".repeat(64);
	assert.equal(reconcileChecksums("asset", pin, pin), pin);
	assert.equal(reconcileChecksums("asset", pin, null), pin);
	assert.equal(reconcileChecksums("asset", null, other), other);
	assert.equal(reconcileChecksums("asset", null, null), null);
	assert.throws(
		() => reconcileChecksums("asset", pin, other),
		/checksum channels disagree/,
	);
});
