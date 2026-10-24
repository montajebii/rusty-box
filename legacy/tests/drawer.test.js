'use strict';
// drawer.test.js — the ideas drawer: open/close semantics, a11y state, search over
// text and bucket names, empty-state messaging, Escape handling.
const test = require('node:test');
const assert = require('node:assert/strict');

const { bootApp, tick } = require('./helpers/boot');

const IDEA = (over = {}) => ({
  id: 'seed-1', text: 'seeded idea', color: '#f0b429',
  created: 1_700_000_000_000, lastViewed: 1_700_000_000_000, buckets: [], ...over,
});

const SEED = [
  IDEA({ id: 'a', text: 'learn todo app patterns', buckets: ['Work'] }),
  IDEA({ id: 'b', text: 'workout plan', buckets: ['Health'] }),
  IDEA({ id: 'c', text: 'totally unrelated thought' }),
];

test('open/close drawer toggles classes, scrim and aria state', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    assert.ok(d.drawerOpen());
    assert.ok(d.$('scrim').classList.contains('open'));
    assert.equal(d.$('drawer').getAttribute('aria-hidden'), 'false');
    assert.ok(d.doc.body.classList.contains('app--modal-open'));

    d.$('closeBtn').click();
    await tick();
    assert.ok(!d.drawerOpen());
    assert.equal(d.$('drawer').getAttribute('aria-hidden'), 'true');
    assert.ok(!d.$('scrim').classList.contains('open'));
    assert.ok(!d.doc.body.classList.contains('app--modal-open'));
  } finally { d.close(); }
});

test('scrim click closes the drawer', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    d.$('scrim').click();
    await tick();
    assert.ok(!d.drawerOpen());
  } finally { d.close(); }
});

test('Escape closes the drawer', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    d.doc.dispatchEvent(new d.win.KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await tick();
    assert.ok(!d.drawerOpen());
  } finally { d.close(); }
});

test('closing the drawer clears the search query and restores the list', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    d.$('searchInput').value = 'workout';
    d.$('searchInput').dispatchEvent(new d.win.Event('input', { bubbles: true }));
    await tick();
    assert.deepEqual(d.cardTexts(), ['workout plan']);

    d.$('closeBtn').click();
    await tick();
    assert.equal(d.$('searchInput').value, '', 'query reset on close');
    assert.equal(d.cards().length, 3, 'full list restored');
  } finally { d.close(); }
});

test('search matches idea text case-insensitively', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    d.$('searchInput').value = '  WORKOUT  ';
    d.$('searchInput').dispatchEvent(new d.win.Event('input', { bubbles: true }));
    await tick();
    assert.deepEqual(d.cardTexts(), ['workout plan']);
  } finally { d.close(); }
});

test('search also matches bucket names', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    d.$('searchInput').value = 'health';
    d.$('searchInput').dispatchEvent(new d.win.Event('input', { bubbles: true }));
    await tick();
    assert.deepEqual(d.cardTexts(), ['workout plan']);
  } finally { d.close(); }
});

test('search narrows the glass counter too', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    d.$('searchInput').value = 'todo';
    d.$('searchInput').dispatchEvent(new d.win.Event('input', { bubbles: true }));
    await tick();
    assert.equal(d.count(), 1);
  } finally { d.close(); }
});

test('empty search result shows a helpful empty state', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    d.$('searchInput').value = 'zzz-nothing';
    d.$('searchInput').dispatchEvent(new d.win.Event('input', { bubbles: true }));
    await tick();
    assert.match(d.$('ideaList').textContent, /Nothing matches/);
    assert.ok(d.$('ideaList').textContent.includes('“zzz-nothing”'));
  } finally { d.close(); }
});

test('empty bucket shows a filing hint instead of nothing', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    d.bucketChips().find((c) => c.dataset.bucket === 'Work').click();
    await tick();
    // delete the only Work member's filing via editor later; here filter with search
    d.$('searchInput').value = 'zzz';
    d.$('searchInput').dispatchEvent(new d.win.Event('input', { bubbles: true }));
    await tick();
    assert.match(d.$('ideaList').textContent, /Nothing in “Work”/);
    assert.match(d.$('ideaList').textContent, /matches “zzz”/);
  } finally { d.close(); }
});

test('bucket filter chips: All ↔ one bucket, with counts', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    const allChip = d.$('bucketBar').querySelector('[data-all]');
    assert.equal(allChip.querySelector('.chip-count').textContent, '3');

    d.bucketChips().find((c) => c.dataset.bucket === 'Health').click();
    await tick();
    assert.deepEqual(d.cardTexts(), ['workout plan']);
    assert.match(d.$('countLabel').textContent, /in “Health”/);

    // the bar re-renders on every filter change — re-query the fresh chip
    d.bucketChips().find((c) => c.dataset.bucket === 'Health').click();
    await tick();
    assert.equal(d.cards().length, 3);
  } finally { d.close(); }
});

test('cards carry buckets chips and a copy/edit/delete/open action row', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    const card = d.cards()[0];
    for (const cls of ['.edit-btn', '.copy-btn', '.view-btn', '.del-btn']) {
      assert.ok(card.querySelector(cls), `card must contain ${cls}`);
    }
    const chips = [...card.querySelectorAll('.bucket-chips .chip-label')].map((c) => c.textContent);
    assert.deepEqual(chips, ['Work']);
  } finally { d.close(); }
});

test('copy action writes the idea text to the clipboard and confirms', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    d.cards()[0].querySelector('.copy-btn').click();
    await tick(10);
    assert.deepEqual(d.hooks.clipboard, ['learn todo app patterns']);
    assert.match(d.toastText(), /Copied to clipboard/);
  } finally { d.close(); }
});
