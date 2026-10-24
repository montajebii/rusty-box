'use strict';
// idea-page.test.js — the full-screen idea page: view mode, the edit flow, draft
// bucket toggling, the close guard (unsaved changes must never be silently lost),
// deletion, share fallback, and Escape scoping.
const test = require('node:test');
const assert = require('node:assert/strict');

const { bootApp, tick } = require('./helpers/boot');

const IDEA = (over = {}) => ({
  id: 'seed-1', text: 'seeded idea', color: '#f0b429',
  created: 1_700_000_000_000, lastViewed: 1_700_000_000_000, buckets: [], ...over,
});

const SEED = [
  IDEA({ id: 'a', text: 'first idea', buckets: ['Work'] }),
  IDEA({ id: 'b', text: 'second idea', buckets: ['Health'] }),
  IDEA({ id: 'c', text: 'third idea' }),
];

async function openPageFor(d, id) {
  // the main list renders inside the drawer, so open it before grabbing a card
  if (!d.drawerOpen()) await d.openDrawer();
  const card = d.cards().find((c) => c.dataset.id === id);
  card.querySelector('.view-btn').click();
  await tick(20);
  return d.ideaPageCard();
}

test('view mode renders text, buckets, timestamps and a real id', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    const card = await openPageFor(d, 'a');
    assert.match(card.querySelector('.idea-page-title').textContent, /first idea/);
    const chips = [...card.querySelectorAll('.idea-page-buckets .chip-label')].map((c) => c.textContent);
    assert.deepEqual(chips, ['Work']);
    assert.ok(card.querySelector('.idea-page-id').textContent.length > 0);
    assert.ok(!card.textContent.includes('Invalid'), 'no "Invalid Date" anywhere');
    assert.ok(card.querySelector('[data-edit]'), 'Edit button present');
    assert.ok(card.querySelector('[data-share]'), 'Share button present');
  } finally { d.close(); }
});

test('unfiled idea shows the inbox hint in view mode', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    const card = await openPageFor(d, 'c');
    assert.match(card.querySelector('.idea-page-buckets').textContent, /no bucket yet/);
  } finally { d.close(); }
});

test('editing saves trimmed text, stamps edited, and returns to view mode', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    const card = await openPageFor(d, 'a');
    card.querySelector('[data-edit]').click();
    await tick();
    const area = card.querySelector('.idea-page-edit');
    assert.ok(area, 'editor visible');
    assert.equal(area.value, 'first idea');

    area.value = '  rewritten idea  ';
    area.dispatchEvent(new d.win.Event('input', { bubbles: true }));
    card.querySelector('[data-save]').click();
    await tick();

    assert.equal(d.ideas().find((i) => i.id === 'a').text, 'rewritten idea');
    assert.ok(d.ideas().find((i) => i.id === 'a').edited, 'edited timestamp set');
    assert.match(d.toastText(), /Idea updated/);
    assert.ok(card.querySelector('.idea-page-title'), 'back in view mode');
  } finally { d.close(); }
});

test('saving an empty edit is rejected with a toast and keeps the old text', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    const card = await openPageFor(d, 'a');
    card.querySelector('[data-edit]').click();
    await tick();
    const area = card.querySelector('.idea-page-edit');
    area.value = '   ';
    area.dispatchEvent(new d.win.Event('input', { bubbles: true }));
    assert.equal(card.querySelector('[data-save]').disabled, true,
      'Save is disabled while the text is empty');
    // the keyboard path (Ctrl/Cmd+Enter) still routes through the guard
    area.dispatchEvent(new d.win.KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true, bubbles: true }));
    await tick();
    assert.match(d.toastText(), /can’t be empty/);
    assert.equal(d.ideas().find((i) => i.id === 'a').text, 'first idea');
    assert.ok(card.querySelector('.idea-page-edit'), 'still editing');
  } finally { d.close(); }
});

test('Cancel discards text and draft bucket changes', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    const card = await openPageFor(d, 'a');
    card.querySelector('[data-edit]').click();
    await tick();
    const area = card.querySelector('.idea-page-edit');
    area.value = 'discarded draft';
    card.querySelector('[data-toggle-bucket="Health"]').click();
    await tick();
    card.querySelector('[data-cancel]').click();
    await tick();
    assert.equal(d.ideas().find((i) => i.id === 'a').text, 'first idea');
    assert.deepEqual(d.ideas().find((i) => i.id === 'a').buckets, ['Work']);
  } finally { d.close(); }
});

test('editor exposes existing buckets plus a new-bucket button', async () => {
  const d = await bootApp({ seed: SEED, promptResult: 'Fresh' });
  try {
    const card = await openPageFor(d, 'a');
    card.querySelector('[data-edit]').click();
    await tick();
    const names = [...card.querySelectorAll('[data-toggle-bucket]')].map((b) => b.getAttribute('data-toggle-bucket'));
    assert.deepEqual(names.sort(), ['Health', 'Work']);
    card.querySelector('[data-new-bucket-editor]').click();
    await tick();
    const now = [...card.querySelectorAll('[data-toggle-bucket]')].map((b) => b.getAttribute('data-toggle-bucket'));
    assert.ok(now.includes('Fresh'), 'new bucket appears as a toggle');
    assert.ok(card.querySelector('[data-toggle-bucket="Fresh"]').getAttribute('aria-pressed') === 'true',
      'created buckets are armed for this idea');
  } finally { d.close(); }
});

test('REGRESSION: confirming “File this idea before closing?” must actually file it', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    const card = await openPageFor(d, 'a');
    card.querySelector('[data-edit]').click();
    await tick();
    card.querySelector('[data-toggle-bucket="Health"]').click();   // draft = [Work, Health]
    await tick();
    d.hooks.confirmResult = true;                                   // user says: file it!
    d.ideaPage().querySelector('.idea-page-scrim').click();
    await tick(320);
    assert.equal(d.ideas().find((i) => i.id === 'a').buckets.includes('Health'), true,
      'confirmed bucket change must be saved, not dropped');
  } finally { d.close(); }
});

test('REGRESSION: swapping buckets (same count) must still be guarded', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    const card = await openPageFor(d, 'a');
    card.querySelector('[data-edit]').click();
    await tick();
    card.querySelector('[data-toggle-bucket="Work"]').click();      // off
    card.querySelector('[data-toggle-bucket="Health"]').click();    // on  → draft [Health], same length
    await tick();
    d.hooks.confirmResult = true;
    d.ideaPage().querySelector('.idea-page-scrim').click();
    await tick(320);
    const guardAsked = d.hooks.confirmCalls.length > 0;
    assert.ok(guardAsked, 'a same-length bucket swap is still an unsaved change — guard it');
    assert.deepEqual(d.ideas().find((i) => i.id === 'a').buckets, ['Health'],
      'confirmed swap must be saved');
  } finally { d.close(); }
});

test('REGRESSION: removing every bucket must still be guarded', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    const card = await openPageFor(d, 'a');
    card.querySelector('[data-edit]').click();
    await tick();
    card.querySelector('[data-toggle-bucket="Work"]').click();      // draft = []
    await tick();
    d.hooks.confirmResult = false;                                  // user declines → discard
    card.querySelector('.idea-page-close').click();
    await tick(320);
    assert.ok(d.hooks.confirmCalls.length > 0, 'emptying the draft must raise the guard');
    assert.deepEqual(d.ideas().find((i) => i.id === 'a').buckets, ['Work'],
      'declined changes are discarded — original filing intact');
  } finally { d.close(); }
});

test('declining the guard discards changes and closes without saving', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    const card = await openPageFor(d, 'a');
    card.querySelector('[data-edit]').click();
    await tick();
    card.querySelector('.idea-page-edit').value = 'unsaved words';
    card.querySelector('.idea-page-edit').dispatchEvent(new d.win.Event('input', { bubbles: true }));
    card.querySelector('[data-toggle-bucket="Health"]').click();
    await tick();
    d.hooks.confirmResult = false;
    d.ideaPage().querySelector('.idea-page-scrim').click();
    await tick(320);
    assert.equal(d.ideas().find((i) => i.id === 'a').text, 'first idea');
    assert.deepEqual(d.ideas().find((i) => i.id === 'a').buckets, ['Work']);
    assert.equal(d.ideaPage(), null, 'overlay gone');
  } finally { d.close(); }
});

test('no guard and no confirm when nothing changed', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    let card = await openPageFor(d, 'a');
    card.querySelector('.idea-page-close').click();   // never entered edit
    await tick(320);
    assert.deepEqual(d.hooks.confirmCalls, []);
    card = await openPageFor(d, 'b');
    card.querySelector('[data-edit]').click();
    await tick();
    card.querySelector('[data-cancel]').click();
    await tick();
    card.querySelector('.idea-page-close').click();
    await tick(320);
    assert.deepEqual(d.hooks.confirmCalls, [], 'edit + cancel + close asks nothing');
  } finally { d.close(); }
});

test('deleting from the idea page removes it and closes the overlay', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    const card = await openPageFor(d, 'b');
    card.querySelector('[data-delete]').click();
    await tick(320);
    assert.equal(d.ideas().length, 2);
    assert.ok(!d.ideas().some((i) => i.id === 'b'));
    assert.equal(d.ideaPage(), null);
    assert.match(d.toastText(), /Idea removed/);
  } finally { d.close(); }
});

test('share falls back to the clipboard when the share API is absent', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    const card = await openPageFor(d, 'a');
    card.querySelector('[data-share]').click();
    await tick(20);
    assert.equal(d.hooks.clipboard.length, 1);
    assert.match(d.hooks.clipboard[0], /Check out my idea in The Idea Bucket: https:\/\/ideabucket\.app\/idea\/a/);
    assert.match(d.toastText(), /Share link copied/);
  } finally { d.close(); }
});

test('copy button on the idea page copies the current text', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    const card = await openPageFor(d, 'a');
    card.querySelector('[data-copy]').click();
    await tick(10);
    assert.deepEqual(d.hooks.clipboard, ['first idea']);
  } finally { d.close(); }
});

test('Escape closes the idea page but not the drawer underneath', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    const card = await openPageFor(d, 'a');
    card.dispatchEvent(new d.win.KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await tick(320);
    assert.equal(d.ideaPage(), null, 'idea page closed');
    assert.ok(d.drawerOpen(), 'drawer stays open');
  } finally { d.close(); }
});

test('REGRESSION: closing the idea page must not drop the drawer’s modal-lock class', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await d.openDrawer();
    await openPageFor(d, 'a');
    await d.closeIdeaPage();
    assert.ok(d.drawerOpen(), 'drawer still open');
    assert.ok(d.doc.body.classList.contains('app--modal-open'),
      'app--modal-open must survive while the drawer below is open');
  } finally { d.close(); }
});

test('opening a second idea page never stacks two overlays', async () => {
  const d = await bootApp({ seed: SEED });
  try {
    await openPageFor(d, 'a');
    const card2 = await openPageFor(d, 'b');
    assert.equal(d.doc.querySelectorAll('.idea-page').length, 1);
    assert.match(card2.querySelector('.idea-page-title').textContent, /second idea/);
  } finally { d.close(); }
});
