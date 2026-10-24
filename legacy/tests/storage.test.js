'use strict';
// storage.test.js — persistence layer: round-trips, filtering of junk entries,
// normalization of legacy/partial ideas, and behaviour when stored data is corrupt
// (a truncated or non-JSON payload must never be silently destroyed).
const test = require('node:test');
const assert = require('node:assert/strict');

const { bootApp, STORAGE_KEY, tick } = require('./helpers/boot');

const IDEA = (over = {}) => ({
  id: 'seed-1', text: 'seeded idea', color: '#f0b429',
  created: 1_700_000_000_000, lastViewed: 1_700_000_000_000, buckets: [], ...over,
});

test('round-trip: ideas survive a reload', async () => {
  let d = await bootApp({ seed: [IDEA(), IDEA({ id: 'seed-2', text: 'second' })] });
  try {
    assert.equal(d.count(), 2);
    assert.equal(d.ideas().length, 2);
  } finally { d.close(); }

  // second boot on the same payload reads the same data back
  d = await bootApp({ seed: d.ideas() });
  try {
    assert.equal(d.count(), 2);
    await d.openDrawer();   // cards render inside the drawer
    assert.deepEqual(d.cards().map((c) => c.dataset.id).sort(), ['seed-1', 'seed-2']);
  } finally { d.close(); }
});

test('junk entries are filtered out; only ideas with string text survive', async () => {
  const d = await bootApp({
    seed: [
      IDEA(),
      null,
      42,
      'a string',
      { noText: true },
      { text: 5 },
    ],
  });
  try {
    assert.equal(d.count(), 1, 'memory holds only the valid idea');
    // normalization reaches storage on the next save
    await d.type('another');
    await d.drop();
    assert.equal(d.ideas().length, 2, 'persisted list has no junk entries');
  } finally { d.close(); }
});

test('bucket lists are cleaned: non-strings dropped, whitespace collapsed, deduped', async () => {
  const d = await bootApp({
    seed: [IDEA({ buckets: ['Work', 'Work', '  Home  ', 'A   B', 7, null, ''] })],
  });
  try {
    await d.type('trigger save');
    await d.drop();
    const [idea] = d.ideas().filter((i) => i.id === 'seed-1');
    assert.deepEqual(idea.buckets, ['Work', 'Home', 'A B']);
  } finally { d.close(); }
});

test('bucket names are capped at 24 characters', async () => {
  const d = await bootApp({ seed: [IDEA({ buckets: ['x'.repeat(40)] })] });
  try {
    await d.type('trigger save');
    await d.drop();
    assert.equal(d.ideas().find((i) => i.id === 'seed-1').buckets[0].length, 24);
  } finally { d.close(); }
});

test('REGRESSION: corrupt JSON is quarantined, not destroyed', async () => {
  const corrupt = '{"ideas": [ {"text": "half written"';   // truncated payload
  const d = await bootApp({ seedRaw: corrupt });
  try {
    assert.equal(d.count(), 0, 'corrupt payload must not crash the app');

    // a later save starts a fresh list — the corrupt bytes must be recoverable
    await d.type('new idea after crash');
    await d.drop();
    assert.equal(d.ideas().length, 1);
    const backup = d.win.localStorage.getItem(STORAGE_KEY + '.corrupt-backup');
    assert.equal(backup, corrupt, 'corrupt payload must be backed up before it is overwritten');
  } finally { d.close(); }
});

test('REGRESSION: valid-JSON-but-not-an-array is quarantined too', async () => {
  const weird = '{"just": "an object"}';
  const d = await bootApp({ seedRaw: weird });
  try {
    assert.equal(d.count(), 0);
    const backup = d.win.localStorage.getItem(STORAGE_KEY + '.corrupt-backup');
    assert.equal(backup, weird);
  } finally { d.close(); }
});

test('REGRESSION: legacy idea without id/color/created is normalized on load', async () => {
  const d = await bootApp({ seed: [{ text: 'ghost idea', buckets: ['Work'] }] });
  try {
    // in-memory normalisation is visible immediately in the rendered card
    await d.openDrawer();
    const dom = d.cards()[0];
    assert.ok(!dom.textContent.includes('Invalid'), 'card must not render "Invalid Date"');
    const dot = dom.querySelector('.dot');
    assert.match(dot.style.background, /rgb\(|#/, 'dot colour must be a real colour');
    assert.ok(dom.dataset.id && dom.dataset.id !== 'undefined', 'card carries a real id');

    // normalization reaches storage on the next save
    await d.type('trigger save');
    await d.drop();
    const [idea] = d.ideas().filter((i) => i.text === 'ghost idea');
    assert.equal(typeof idea.id, 'string', 'id must be generated');
    assert.ok(idea.id.length > 0);
    assert.match(idea.color, /^#[0-9a-fA-F]{6}$/, 'color must be a usable hex');
    assert.equal(typeof idea.created, 'number', 'created must be a number');

    // and the idea can be deleted by its (now real) id
    [...d.cards()].find((c) => c.textContent.includes('ghost idea')).querySelector('.del-btn').click();
    await tick();
    assert.equal(d.ideas().filter((i) => i.text === 'ghost idea').length, 0,
      'delete must work for normalized legacy ideas');
  } finally { d.close(); }
});

test('deleting an idea persists the removal', async () => {
  const d = await bootApp({ seed: [IDEA(), IDEA({ id: 'seed-2', text: 'two' })] });
  try {
    await d.openDrawer();
    d.cards()[0].querySelector('.del-btn').click();
    await tick();
    assert.equal(d.ideas().length, 1);
    assert.equal(d.count(), 1);
  } finally { d.close(); }
});

test('save failure (quota) shows a toast instead of throwing', async () => {
  const d = await bootApp({ seed: [IDEA()] });
  try {
    // jsdom ignores method shadowing on its Storage instance, so swap the
    // whole property for a storage that is out of quota.
    const fullStorage = {
      getItem: () => null, removeItem: () => {}, key: () => null, clear: () => {},
      length: 0,
      setItem: () => { throw new Error('QuotaExceededError'); },
    };
    Object.defineProperty(d.win, 'localStorage', { value: fullStorage, configurable: true });
    await d.type('will not persist');
    await d.drop();
    assert.ok(d.toastShown(), 'toast visible');
    assert.match(d.toastText(), /Storage full/);
    // the in-memory list still grew, so the session keeps working
    assert.equal(d.count(), 2);
  } finally { d.close(); }
});
