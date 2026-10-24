'use strict';
// static.test.js — structural invariants that need no app execution:
// the inline script parses, every id the script looks up exists, no duplicate
// ids, package metadata is sane, workflows are valid YAML, shell scripts parse.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const { execFileSync } = require('node:child_process');
const { parse } = require('yaml');

const { ROOT, bootApp } = require('./helpers/boot');

const html = fs.readFileSync(path.join(ROOT, 'index.html'), 'utf8');

function inlineScripts(source) {
  return [...source.matchAll(/<script(?![^>]*\bsrc=)[^>]*>([\s\S]*?)<\/script>/gi)].map((m) => m[1]);
}

test('index.html contains exactly one inline script block', () => {
  const blocks = inlineScripts(html);
  assert.equal(blocks.length, 1, 'expected the single app <script>');
});

test('inline JavaScript is syntactically valid (node --check)', () => {
  const [script] = inlineScripts(html);
  assert.ok(script && script.trim().length > 0, 'inline script must not be empty');
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'idea-bucket-'));
  const file = path.join(dir, 'inline.js');
  fs.writeFileSync(file, script);
  execFileSync(process.execPath, ['--check', file]); // throws on syntax error
  fs.rmSync(dir, { recursive: true, force: true });
});

test('every id referenced by the script exists in the static DOM', async () => {
  const [script] = inlineScripts(html);
  const ids = new Set();
  for (const m of script.matchAll(/\$\('([A-Za-z][\w-]*)'\)/g)) ids.add(m[1]);
  for (const m of script.matchAll(/getElementById\('([A-Za-z][\w-]*)'\)/g)) ids.add(m[1]);
  assert.ok(ids.size >= 20, `expected a healthy number of id lookups, saw ${ids.size}`);

  const d = await bootApp();
  try {
    for (const id of ids) {
      assert.ok(d.$(id), `script references #${id} but no such element exists`);
    }
  } finally { d.close(); }
});

test('no duplicate element ids in the document', async () => {
  const d = await bootApp();
  try {
    const seen = new Map();
    for (const el of d.doc.querySelectorAll('[id]')) {
      const id = el.id;
      assert.ok(!seen.has(id), `duplicate id #${id}`);
      seen.set(id, el.tagName);
    }
  } finally { d.close(); }
});

test('script never leaves an unbalanced template literal (smoke: app boots and renders)', async () => {
  const d = await bootApp();
  try {
    assert.equal(d.count(), 0);
    assert.equal(d.text(d.$('countLabel')), 'ideas in the bucket');
    assert.match(d.$('composerBuckets').textContent, /no bucket yet/);
  } finally { d.close(); }
});

test('package.json: version is semver and npm test is wired up', () => {
  const pkg = JSON.parse(fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8'));
  assert.match(pkg.version, /^\d+\.\d+\.\d+$/, 'package.json version must be semver');
  assert.ok(pkg.scripts && typeof pkg.scripts.test === 'string', 'scripts.test missing');
  assert.match(pkg.scripts.test, /node --test/, 'npm test should run the node test runner');
});

test('capacitor.config.json appId is stable', () => {
  const cfg = JSON.parse(fs.readFileSync(path.join(ROOT, 'capacitor.config.json'), 'utf8'));
  assert.equal(cfg.appId, 'com.ideabucket.app');
});

test('GitHub workflows are valid YAML with expected triggers', () => {
  for (const wf of ['build.yml', 'release.yml']) {
    const doc = parse(fs.readFileSync(path.join(ROOT, '.github/workflows', wf), 'utf8'));
    assert.ok(doc.name, `${wf}: missing workflow name`);
    assert.ok(doc.jobs && Object.keys(doc.jobs).length > 0, `${wf}: no jobs`);
    assert.ok(doc.on, `${wf}: missing trigger`);
  }
  const build = parse(fs.readFileSync(path.join(ROOT, '.github/workflows/build.yml'), 'utf8'));
  assert.ok(build.jobs.web, 'build.yml should keep the web-checks job');
  assert.ok(build.jobs.apk, 'build.yml should keep the apk job');
});

test('shell scripts pass bash -n (syntax)', () => {
  for (const s of ['build-apk.sh', 'build-env.sh', 'check-apk.sh', 'make-release.sh']) {
    const file = path.join(ROOT, s);
    assert.ok(fs.existsSync(file), `${s} missing`);
    execFileSync('bash', ['-n', file]); // throws on syntax error
  }
});
