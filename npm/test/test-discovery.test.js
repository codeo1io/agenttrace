'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync, existsSync, readdirSync } = require('node:fs');
const path = require('node:path');

// rm-1869: `node --test test/**/*.test.js` expands the glob only on
// Node >= 21 — on the engines floor the pattern stays literal, zero
// test files run, and the suite still exits green (a zero-match pass
// observed live in this cycle's assess). The test script therefore
// enumerates every file explicitly, and this guard pins that contract
// so the glob can never quietly return.
//
// Review-fix F2: explicit enumeration makes every NEW test file
// opt-in — the old glob auto-covered new files on Node >= 21, so a
// file the script forgot to list would silently never run on ANY
// Node version. The guard is therefore set-equal to the directory: a
// *.test.js on disk that the script does not list fails here.
test('npm test enumerates every test file explicitly (rm-1869 guard)', () => {
  const pkgRoot = path.join(__dirname, '..');
  const pkg = JSON.parse(readFileSync(path.join(pkgRoot, 'package.json'), 'utf8'));
  const script = pkg.scripts && pkg.scripts.test;
  assert.equal(typeof script, 'string', 'package.json must declare scripts.test');
  assert.match(script, /^node --test /, `test script must invoke the runner directly: ${script}`);

  const listed = script.replace(/^node --test /, '').split(/\s+/).filter(Boolean);
  assert.ok(listed.length > 0, 'test script must enumerate at least one file');

  for (const entry of listed) {
    assert.ok(!/[*?[\]]/.test(entry), `glob metacharacter leaked back into the test script: ${entry}`);
    assert.equal(
      path.dirname(entry),
      'test',
      `test script entries must live under test/: ${entry}`
    );
    assert.ok(
      existsSync(path.join(pkgRoot, entry)),
      `test script lists a file that does not exist: ${entry}`
    );
  }

  // This guard must itself be covered by the enumeration.
  assert.ok(
    listed.some((entry) => entry.endsWith('test-discovery.test.js')),
    'test-discovery.test.js must be listed in scripts.test'
  );

  // Completeness (review-fix F2): the enumeration is exactly the set
  // of *.test.js files present in npm/test/ — no forgotten file, no
  // stale entry, on every Node version.
  const onDisk = readdirSync(path.join(pkgRoot, 'test'))
    .filter((name) => name.endsWith('.test.js'))
    .sort();
  const listedNames = listed.map((entry) => path.basename(entry)).sort();
  assert.deepEqual(
    listedNames,
    onDisk,
    'scripts.test must enumerate exactly the *.test.js files present in test/'
  );
});
