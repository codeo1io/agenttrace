'use strict';

// rm-1002 contract: the npm installer's download transport is
// scheme-pinned and byte-capped.
//
// Pre-fix, install.js followed ANY redirect scheme (an https hop could
// hand the download to `http://` — or anything node can name) and piped
// the response to disk with no size bound, so the abort-before-
// checksum property did not exist: a hostile or corrupt asset could
// stream unbounded bytes into the npm cache directory before the
// checksum comparison ever ran.
//
// These tests exercise install.js as a MODULE (the verify-runtime.js
// offline pattern), injecting `http.get` as the requester so the
// redirect/limit logic runs against real local sockets — no network,
// no npm pack, no release assets. Boundary arms per
// docs/quality-gates/byte-caps-must-test-the-eof-edge.md: cap+1 aborts,
// exactly-cap completes.
//
// The postinstall entry point stays `main()` guarded by
// `require.main === module`, so `npm pack`+install behavior is
// unchanged (scripts/install.js is still a script first).

const assert = require('node:assert/strict');
const { existsSync, mkdtempSync, rmSync, statSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join } = require('node:path');
const { Readable } = require('node:stream');
const http = require('node:http');
const test = require('node:test');

const {
	download,
	resolveRedirectTarget,
	MAX_DOWNLOAD_BYTES,
} = require(join(__dirname, '..', 'scripts', 'install.js'));

function withServer(handler, run) {
	return new Promise((resolve, reject) => {
		const server = http.createServer(handler);
		server.on('error', reject);
		server.listen(0, '127.0.0.1', () => {
			Promise.resolve(run(server))
				.then(
					(value) => server.close(() => resolve(value)),
					(error) => server.close(() => reject(error)),
				);
		});
	});
}

function scratchDest(tag) {
	const dir = mkdtempSync(join(tmpdir(), `agenttrace-transport-${tag}-`));
	return { dir, destination: join(dir, 'bin') };
}

test('the default download cap is 128 MiB', () => {
	assert.equal(MAX_DOWNLOAD_BYTES, 128 * 1024 * 1024);
});

test('redirect targets are https-only', () => {
	const base = 'https://github.com/codeo1io/agenttrace/releases/latest/download/x';
	assert.equal(
		resolveRedirectTarget('https://objects.example/asset', base).href,
		'https://objects.example/asset',
	);
	assert.equal(
		resolveRedirectTarget('/v1/asset', base).href,
		'https://github.com/v1/asset',
		'relative locations resolve against the current hop',
	);
	const downgrades = [
		'http://downgrade.example/asset',
		'file:///etc/passwd',
		'ftp://mirror.example/asset',
		'gopher://127.0.0.1:7070/asset',
	];
	for (const location of downgrades) {
		assert.throws(
			() => resolveRedirectTarget(location, base),
			/unsupported scheme/,
			`scheme-pinning must refuse ${location}`,
		);
	}
});

test('download refuses a non-https redirect Location mid-chain', async () => {
	await withServer((request, response) => {
		// An https hop handing us off to a plaintext host — the classic
		// on-path downgrade. The refusal must name the scheme.
		response.statusCode = 302;
		response.setHeader('Location', 'http://127.0.0.1:9/evil/asset');
		response.end();
	}, async (server) => {
		const port = server.address().port;
		const { dir, destination } = scratchDest('scheme');
		await assert.rejects(
			download(`http://127.0.0.1:${port}/release`, destination, 0, {
				request: http.get,
			}),
			/unsupported scheme/,
		);
		assert.equal(existsSync(destination), false, 'no partial file left behind');
		rmSync(dir, { recursive: true, force: true });
	});
});

test('download aborts a body over the byte cap before checksum verification', async () => {
	const cap = 1024;
	await withServer((request, response) => {
		response.statusCode = 200;
		response.end(Buffer.alloc(cap + 1, 0x61)); // cap+1 bytes, then EOF
	}, async (server) => {
		const port = server.address().port;
		const { dir, destination } = scratchDest('over');
		await assert.rejects(
			download(`http://127.0.0.1:${port}/big`, destination, 0, {
				request: http.get,
				maxBytes: cap,
			}),
			/byte cap/,
		);
		assert.equal(existsSync(destination), false, 'partial download removed on abort');
		rmSync(dir, { recursive: true, force: true });
	});
});

test('download completes a body exactly at the byte cap (EOF edge)', async () => {
	const cap = 1024;
	await withServer((request, response) => {
		response.statusCode = 200;
		response.end(Buffer.alloc(cap, 0x61)); // exactly cap bytes, then EOF
	}, async (server) => {
		const port = server.address().port;
		const { dir, destination } = scratchDest('edge');
		await download(`http://127.0.0.1:${port}/edge`, destination, 0, {
			request: http.get,
			maxBytes: cap,
		});
		assert.equal(
			statSync(destination).size,
			cap,
			'cap-sized bodies must complete — the cap only fires on cap+1',
		);
		rmSync(dir, { recursive: true, force: true });
	});
});

test('download still follows scheme-valid redirects', async () => {
	// Positive control for the redirect path itself: a 302 to an
	// https:// location is followed. `request` is injected, but scheme
	// validation runs on the TARGET: the local hop answers 302, and the
	// fake arm records that the second request really went to the
	// https:// location named in the Location header (no real network:
	// example.invalid is RFC 2606 reserved).
	const cap = 64;
	const hops = [];
	const body = Buffer.alloc(cap, 0x62);
	const requester = (url, options, callback) => {
		const target = String(url);
		hops.push(target);
		if (target.startsWith('http://')) return http.get(url, options, callback);
		const response = new Readable({ read() {} });
		response.statusCode = 200;
		response.headers = {};
		process.nextTick(() => {
			response.push(body);
			response.push(null);
			callback(response);
		});
		return { on() {}, destroy() {} };
	};
	await withServer((request, response) => {
		response.statusCode = 302;
		response.setHeader('Location', 'https://example.invalid/second');
		response.end();
	}, async (server) => {
		const port = server.address().port;
		const { dir, destination } = scratchDest('follow');
		await download(`http://127.0.0.1:${port}/first`, destination, 0, {
			request: requester,
			maxBytes: cap,
		});
		assert.equal(statSync(destination).size, cap, 'redirect was followed to the body');
		assert.equal(
			hops[hops.length - 1],
			'https://example.invalid/second',
			'the second hop used the validated https target',
		);
		rmSync(dir, { recursive: true, force: true });
	});
});
