'use strict';
// scripts.test.js — the release tooling: build-env.sh's toolchain resolution
// contract (sourced, exit codes, exports), and the cheap guards in
// make-release.sh / check-apk.sh that must fail fast before touching gradle.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..');

function forgeToolchain(dir) {
  const sdk = path.join(dir, 'Sdk');
  const bt = path.join(sdk, 'build-tools', '35.0.0');
  const jdk = path.join(dir, 'jdk');
  fs.mkdirSync(bt, { recursive: true });
  fs.mkdirSync(path.join(jdk, 'bin'), { recursive: true });
  for (const tool of [path.join(bt, 'aapt2'), path.join(jdk, 'bin', 'javac')]) {
    fs.writeFileSync(tool, '#!/bin/sh\nexit 0\n');
    fs.chmodSync(tool, 0o755);
  }
  return { sdk, jdk };
}

const cleanEnv = () => ({
  PATH: process.env.PATH,
  HOME: os.tmpdir(),
  // strip any inherited toolchain hints so the fallback paths are exercised
  ANDROID_HOME: '',
  ANDROID_SDK_ROOT: '',
  JAVA_HOME: '',
});

test('build-env.sh: no SDK → clear error, non-zero exit', () => {
  const res = spawnSync('bash', ['-c', '. ./build-env.sh'], {
    cwd: ROOT,
    env: cleanEnv(),
    encoding: 'utf8',
  });
  assert.ok(res.status !== 0, 'sourcing must fail without a toolchain');
  assert.match(res.stderr, /no Android SDK found/);
});

test('build-env.sh: forged SDK + JDK → exports the full toolchain', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ib-toolchain-'));
  const { sdk, jdk } = forgeToolchain(dir);
  const bash = [
    'set -e',
    `export ANDROID_HOME="${sdk}"`,
    `export JAVA_HOME="${jdk}"`,
    '. ./build-env.sh',
    `test "$ANDROID_HOME" = "${sdk}"`,
    `test "$ANDROID_SDK_ROOT" = "${sdk}"`,
    'test -x "$BUILD_TOOLS/aapt2"',
    'case "${GRADLE_JDK_ARGS[0]}" in -Dorg.gradle.java.home=*) ;; *) exit 9;; esac',
    'echo TOOLCHAIN_OK',
  ].join('\n');
  const res = spawnSync('bash', ['-c', bash], { cwd: ROOT, encoding: 'utf8' });
  assert.equal(res.stdout.trim(), 'TOOLCHAIN_OK', res.stderr);
  fs.rmSync(dir, { recursive: true, force: true });
});

test('build-env.sh: SDK without build-tools 35 → named error', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ib-toolchain-'));
  const { sdk, jdk } = forgeToolchain(dir);
  fs.rmSync(path.join(sdk, 'build-tools'), { recursive: true, force: true });
  const env = { ...cleanEnv(), ANDROID_HOME: sdk, JAVA_HOME: jdk };
  const res = spawnSync('bash', ['-c', '. ./build-env.sh'], { cwd: ROOT, env, encoding: 'utf8' });
  assert.ok(res.status !== 0);
  assert.match(res.stderr, /build-tools 35\.0\.0 not found/);
  fs.rmSync(dir, { recursive: true, force: true });
});

test('make-release.sh: rejects non-semver versions before building anything', () => {
  const r = spawnSync('bash', ['make-release.sh', 'not-a-version'], {
    cwd: ROOT, encoding: 'utf8', env: cleanEnv(),
  });
  assert.ok(r.status !== 0);
  assert.match(r.stderr, /not a semver/);
  assert.ok(!fs.existsSync(path.join(ROOT, 'releases')), 'no release dir may be created');
});

test('check-apk.sh: no toolchain → fails fast at the env gate', () => {
  const r = spawnSync('bash', ['check-apk.sh'], {
    cwd: ROOT, encoding: 'utf8', env: cleanEnv(),
  });
  assert.ok(r.status !== 0);
  assert.match(r.stderr, /no Android SDK found/);
});
