#!/usr/bin/env node

const {
	chmodSync,
	createWriteStream,
	existsSync,
	mkdirSync,
	readFileSync,
	renameSync,
	rmSync,
} = require("node:fs");
const { arch, platform } = require("node:process");
const { join } = require("node:path");
const { pipeline } = require("node:stream/promises");
const { get } = require("node:https");
const { createHash } = require("node:crypto");
const { verifyRuntime } = require("./verify-runtime");

const PACKAGE_VERSION = require("../package.json").version;
const REPOSITORY = "luoyuctl/agenttrace";
const ARCHITECTURES = { x64: "amd64", arm64: "arm64" };
const PLATFORMS = { darwin: "darwin", linux: "linux", win32: "windows" };

// rm-1002: hard ceiling for one downloaded release asset. The npm
// cache directory is a hostile place for unbounded streams: without
// this cap the abort-before-checksum property did not exist — a
// hostile or corrupt asset could stream unbounded bytes to disk
// BEFORE the sha256 comparison ever ran. 128 MiB dwarfs the ~15 MiB
// binaries the release publishes.
const MAX_DOWNLOAD_BYTES = 128 * 1024 * 1024;
const MAX_REDIRECTS = 5;

function fail(message) {
	throw new Error(`agenttrace installation failed: ${message}`);
}

function getAssetName() {
	const targetPlatform = PLATFORMS[platform];
	const targetArchitecture = ARCHITECTURES[arch];
	if (!targetPlatform || !targetArchitecture) {
		fail(`unsupported platform: ${platform}/${arch}`);
	}

	return `agenttrace-${targetPlatform}-${targetArchitecture}${
		platform === "win32" ? ".exe" : ""
	}`;
}

// rm-1002: redirect targets must stay on TLS. The old resolver
// followed ANY scheme — an https hop could hand the download to
// `http://` (or any scheme node can name) and the download would
// follow it. Pure validation: returns the resolved absolute URL, or
// throws before any bytes move.
function resolveRedirectTarget(location, current) {
	const target = new URL(location, current);
	if (target.protocol !== "https:") {
		throw new Error(
			`agenttrace installation failed: unsupported scheme ${target.protocol} in redirect ${target.href}`,
		);
	}
	return target;
}

function download(
	url,
	destination,
	redirects = 0,
	{ request = get, maxBytes = MAX_DOWNLOAD_BYTES } = {},
) {
	return new Promise((resolve, reject) => {
		request(
			url,
			{
				headers: {
					Accept: "application/octet-stream",
					"User-Agent": "agenttrace-npm-installer",
				},
			},
			(response) => {
				const location = response.headers.location;
				if (
					response.statusCode >= 300 &&
					response.statusCode < 400 &&
					location &&
					redirects < MAX_REDIRECTS
				) {
					response.resume();
					// rm-1002: scheme-pinned redirect following - the target
					// must stay on https, whatever the Location header says.
					let target;
					try {
						target = resolveRedirectTarget(location, url);
					} catch (error) {
						reject(error);
						return;
					}
					resolve(
						download(target, destination, redirects + 1, { request, maxBytes }),
					);
					return;
				}
				if (response.statusCode >= 300 && response.statusCode < 400) {
					response.resume();
					reject(
						new Error(
							`redirect loop or missing Location at ${url} (after ${redirects} redirects)`,
						),
					);
					return;
				}
				if (response.statusCode !== 200) {
					response.resume();
					reject(new Error(`download returned HTTP ${response.statusCode}`));
					return;
				}
				// rm-1002: count bytes to disk - content-length AND streamed
				// bodies both cross the cap. Over cap: destroy the socket,
				// remove the partial file, refuse. The abort fires BEFORE
				// the checksum comparison ever sees the file (previously
				// the pipeline streamed unbounded bytes into the npm cache).
				const declaredLength = Number(response.headers["content-length"]);
				if (Number.isFinite(declaredLength) && declaredLength > maxBytes) {
					response.destroy();
					rmSync(destination, { force: true });
					reject(
						new Error(
							`release asset declares ${declaredLength} bytes, over the ${maxBytes}-byte cap`,
						),
					);
					return;
				}
				let received = 0;
				response.on("data", (chunk) => {
					received += chunk.length;
					if (received > maxBytes) {
							response.destroy(
								new Error(
									`release asset exceeds the ${maxBytes}-byte cap mid-stream (byte cap breached)`,
								),
						);
					}
				});
				pipeline(response, createWriteStream(destination)).then(
					resolve,
					(error) => {
						rmSync(destination, { force: true });
						const reason =
							error && error.message ? error.message : "stream closed";
						reject(new Error(`byte cap or stream failure: ${reason}`));
					},
				);
			},
		).on("error", reject);
	});
}

async function main() {
	const asset = getAssetName();
	const releaseUrl = `https://github.com/${REPOSITORY}/releases/download/v${PACKAGE_VERSION}/${asset}`;
	const checksumUrl = `${releaseUrl}.sha256`;
	const packageRoot = join(__dirname, "..");
	const libDirectory = join(packageRoot, "lib");
	const temporaryBinary = join(libDirectory, `${asset}.download`);
	const temporaryChecksum = join(libDirectory, `${asset}.sha256`);
	const destination = join(
		libDirectory,
		platform === "win32" ? "agenttrace.exe" : "agenttrace",
	);

	mkdirSync(libDirectory, { recursive: true });
	rmSync(temporaryBinary, { force: true });
	rmSync(temporaryChecksum, { force: true });

	try {
		await Promise.all([
			download(releaseUrl, temporaryBinary),
			download(checksumUrl, temporaryChecksum),
		]);
		const expected = readFileSync(temporaryChecksum, "utf8")
			.trim()
			.split(/\s+/)[0];
		const actual = createHash("sha256")
			.update(readFileSync(temporaryBinary))
			.digest("hex");
		if (expected !== actual) {
			fail(`checksum mismatch for ${asset}`);
		}
		renameSync(temporaryBinary, destination);
		if (platform !== "win32") {
			chmodSync(destination, 0o755);
		}
		// A checksum-valid download can still be incompatible with this host
		// (e.g. built against a newer glibc), so prove the binary actually runs
		// before declaring success — never leave a broken binary behind.
		const verification = verifyRuntime(destination);
		if (!verification.ok) {
			rmSync(destination, { force: true });
			fail(
				`downloaded binary is incompatible with this host and does not run: ${verification.stderrTail}. ` +
						"Install Rust (https://rustup.rs) and build from source instead, see the repository README",
			);
		}
	} finally {
		rmSync(temporaryBinary, { force: true });
		rmSync(temporaryChecksum, { force: true });
	}

	if (!existsSync(destination)) {
		fail(`downloaded binary was not installed: ${asset}`);
	}
}

module.exports = {
	download,
	resolveRedirectTarget,
	MAX_DOWNLOAD_BYTES,
	MAX_REDIRECTS,
};

// Script-first: postinstall still runs this file directly. The
// module guard keeps `require("./scripts/install.js")` (the offline
// tests in test/install-transport.test.js, rm-1002) side-effect free.
if (require.main === module) {
	main().catch((error) => {
		console.error(error.message);
		process.exit(1);
	});
}
