'use strict';
// buckets.test.js — the bucket model exercised through the UI:
// naming rules, case-insensitive reuse, multi-bucket membership, the pending
// ("file into") flow, and bucket deletion (ideas survive, filing does not).
const test = require('node:test');
const assert = require('node:assert/strict');

const { bootApp, tick } = require('./helpers/boot');

const IDEA = (over = {}) => ({
  id: 'seed-1', text: 'seeded idea', color: '#f0b429',
  created: 1_700_000_000_000, lastViewed: 1_700_000_000_000, buckets: [], ...over,
});

test('creating a bucket via the composer chips', async () => {
  const d = await bootApp({ promptResult: 'Work' });
  try {
    await d.addBucketViaPrompt('Work');
    assert.deepEqual(d.composerChipNames(), ['Work']);
    assert.match(d.hooks.promptCalls[0], /Name this bucket/);
    // the chip is armed (active) for the next drop
    const chip = d.composerChips()[0];
    assert.equal(chip.getAttribute('aria-pressed'), 'true');
  } finally { d.close(); }
});

test('bucket names are cleaned: trimmed, whitespace collapsed, 24-char cap', async () => {
  const d = await bootApp({ promptResult: '   Lots   of    spaces   ' });
  try {
    await d.addBucketViaPrompt();
    assert.deepEqual(d.composerChipNames(), ['Lots of spaces']);
  } finally { d.close(); }

  const d2 = await bootApp({ promptResult: 'y'.repeat(40) });
  try {
    await d2.addBucketViaPrompt();
    const [name] = d2.composerChipNames();
    assert.equal(name.length, 24);
  } finally { d2.close(); }
});

test('empty prompt name is rejected with a toast; cancel (null) is silent', async () => {
  let d = await bootApp({ promptResult: '   ' });
  try {
    await d.addBucketViaPrompt();
    assert.ok(d.toastShown());
    assert.match(d.toastText(), /Give the bucket a name/);
    assert.deepEqual(d.composerChipNames(), []);
  } finally { d.close(); }

  d = await bootApp({ promptResult: null });
  try {
    await d.addBucketViaPrompt();
    assert.deepEqual(d.composerChipNames(), []);
    assert.ok(!d.toastShown(), 'cancelling must not nag');
  } finally { d.close(); }
});

test('a bucket name matching an existing bucket only differs by case → reuse it', async () => {
  const d = await bootApp({ seed: [IDEA({ buckets: ['Work'] })], promptResult: 'work' });
  try {
    await d.addBucketViaPrompt();
    assert.deepEqual(d.composerChipNames(), ['Work'], 'must reuse the existing spelling');
    assert.ok(!d.composerChipNames().includes('work'));
  } finally { d.close(); }
});

test('an idea can live in several buckets at once', async () => {
  let d = await bootApp();
  try {
    d.hooks.promptResult = 'A';
    await d.addBucketViaPrompt();
    d.hooks.promptResult = 'B';
    await d.addBucketViaPrompt();
    await d.type('multi bucket idea');
    await d.drop();
    const [idea] = d.ideas();
    assert.deepEqual(idea.buckets, ['A', 'B']);
    assert.match(d.toastText(), /Filed into A \+ B/);
  } finally { d.close(); }

  // and the drawer shows both chips with counts
  d = await bootApp({ seed: [IDEA({ buckets: ['A', 'B'] })] });
  try {
    await d.openDrawer();
    assert.deepEqual(d.bucketChipNames(), ['A', 'B']);
    for (const chip of d.bucketChips()) assert.equal(chip.querySelector('.chip-count').textContent, '1');
  } finally { d.close(); }
});

test('toggling a pending bucket off before dropping files into the inbox', async () => {
  const d = await bootApp({ seed: [IDEA({ buckets: ['Work'] })] });
  try {
    // arm then disarm "Work"
    d.composerChips()[0].click();
    await tick();
    d.composerChips()[0].click();
    await tick();
    await d.type('inbox idea');
    await d.drop();
    assert.deepEqual(d.ideas()[0].buckets, []);
    assert.match(d.toastText(), /inbox/);
  } finally { d.close(); }
});

test('dropping while a drawer filter is active files into that bucket', async () => {
  const d = await bootApp({ seed: [IDEA({ buckets: ['Work'] })] });
  try {
    await d.openDrawer();
    d.bucketChips()[0].click();          // activate the Work filter
    await tick();
    await d.type('filtered drop');
    await d.drop();
    assert.deepEqual(d.ideas().find((i) => i.text === 'filtered drop').buckets, ['Work']);
  } finally { d.close(); }
});

test('deleting a bucket unhooks its ideas but keeps every idea', async () => {
  const d = await bootApp({
    seed: [
      IDEA({ id: 'a', text: 'one', buckets: ['Work', 'Home'] }),
      IDEA({ id: 'b', text: 'two', buckets: ['Work'] }),
      IDEA({ id: 'c', text: 'three', buckets: ['Home'] }),
    ],
  });
  try {
    await d.openDrawer();
    const work = d.bucketChips().find((c) => c.dataset.bucket === 'Work');
    // real gesture sequence: pointerdown (resets the long-press latch) → contextmenu → pointerup
    const holdDelete = () => {
      work.dispatchEvent(new d.win.Event('pointerdown', { bubbles: true }));
      work.dispatchEvent(new d.win.Event('contextmenu', { bubbles: true, cancelable: true }));
      work.dispatchEvent(new d.win.Event('pointerup', { bubbles: true }));
    };
    holdDelete();
    await tick();
    assert.match(d.hooks.confirmCalls.at(-1), /Delete the “Work” bucket/);
    assert.match(d.hooks.confirmCalls.at(-1), /2 ideas/);
    // confirmResult defaults to false → nothing happens
    assert.equal(d.ideas().filter((i) => i.buckets.includes('Work')).length, 2);

    d.hooks.confirmResult = true;
    holdDelete();
    await tick();
    assert.deepEqual(d.ideas().map((i) => i.buckets), [['Home'], [], ['Home']]);
    assert.ok(!d.bucketChipNames().includes('Work'), 'empty bucket disappears from the bar');
    assert.match(d.toastText(), /removed from 2 ideas/);
    // the 3 ideas all still exist
    assert.equal(d.ideas().length, 3);
  } finally { d.close(); }
});

test('deleting the active bucket filter falls back to All', async () => {
  const d = await bootApp({ seed: [IDEA({ buckets: ['Solo'] })] });
  try {
    await d.openDrawer();
    d.bucketChips()[0].click();          // filter by Solo
    await tick();
    assert.match(d.$('countLabel').textContent, /in “Solo”/);
    d.hooks.confirmResult = true;
    d.bucketChips()[0].dispatchEvent(new d.win.Event('contextmenu', { bubbles: true, cancelable: true }));
    await tick();
    assert.equal(d.$('countLabel').textContent, 'idea in the bucket', 'filter fell back to All');
  } finally { d.close(); }
});

test('pending buckets are cleaned when their bucket is deleted', async () => {
  const d = await bootApp({ seed: [IDEA({ buckets: ['Work'] })] });
  try {
    d.hooks.promptResult = null;
    // arm Work as pending
    d.composerChips()[0].click();
    await tick();
    assert.equal(d.composerChips()[0].getAttribute('aria-pressed'), 'true');
    // delete Work via the drawer
    await d.openDrawer();
    d.hooks.confirmResult = true;
    d.bucketChips()[0].dispatchEvent(new d.win.Event('contextmenu', { bubbles: true, cancelable: true }));
    await tick();
    // Work had 1 member; after unfiled it is gone from the index entirely
    assert.ok(!d.composerChipNames().includes('Work'), 'pending bucket must vanish after deletion');
  } finally { d.close(); }
});

test('bucket colours are stable per name (same name → same colour)', async () => {
  const d = await bootApp({ seed: [IDEA({ buckets: ['Stable'] })] });
  try {
    await d.openDrawer();
    const style = d.bucketChips()[0].getAttribute('style');
    d.close();
    const d2 = await bootApp({ seed: [IDEA({ buckets: ['Stable'] })] });
    try {
      await d2.openDrawer();
      assert.equal(d2.bucketChips()[0].getAttribute('style'), style);
    } finally { d2.close(); }
  } finally { d.close(); }
});
