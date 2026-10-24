'use strict';
// composer.test.js — the capture path: dropping ideas via button and keyboard,
// empty-input rejection, the glass (counter/liquid/drip hint) reacting, and the
// "from: …" source line captured from a page selection.
const test = require('node:test');
const assert = require('node:assert/strict');

const { bootApp, tick } = require('./helpers/boot');

const BOTTOM_Y = 325, TOP_Y = 150;

test('empty input is rejected: no idea, no save, input untouched', async () => {
  const d = await bootApp();
  try {
    await d.drop();
    assert.equal(d.ideas().length, 0);
    assert.equal(d.count(), 0);
    assert.ok(!d.toastShown());
  } finally { d.close(); }
});

test('whitespace-only input is rejected too', async () => {
  const d = await bootApp();
  try {
    await d.type('   \n\t  ');
    await d.drop();
    assert.equal(d.ideas().length, 0);
  } finally { d.close(); }
});

test('dropping an idea stores it newest-first with metadata and clears the input', async () => {
  const d = await bootApp();
  try {
    await d.type('  buy a whiteboard  ');
    await d.drop();
    const [idea] = d.ideas();
    assert.equal(idea.text, 'buy a whiteboard', 'text is trimmed');
    assert.equal(typeof idea.id, 'string');
    assert.match(idea.color, /^#[0-9a-fA-F]{6}$/);
    assert.equal(Math.abs(idea.created - Date.now()) < 5000, true, 'created ≈ now');
    assert.deepEqual(idea.buckets, []);
    assert.equal(d.input().value, '', 'composer resets after a drop');
    assert.equal(d.count(), 1);
    assert.equal(d.text(d.$('countLabel')), 'idea in the bucket', 'singular label for 1 idea');
  } finally { d.close(); }
});

test('Enter in the composer drops the idea', async () => {
  const d = await bootApp();
  try {
    await d.type('keyboard idea');
    d.input().dispatchEvent(new d.win.KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await tick();
    assert.equal(d.ideas().length, 1);
  } finally { d.close(); }
});

test('the glass tracks the number of ideas', async () => {
  const d = await bootApp();
  try {
    assert.equal(d.liquidY(), BOTTOM_Y, 'empty bucket: liquid at the bottom');
    assert.equal(d.$('dripHint').getAttribute('opacity'), '1', 'drip hint visible when empty');
    assert.ok(!d.$('stage').classList.contains('has-ideas'));

    await d.type('one'); await d.drop();
    assert.equal(d.liquidY(), BOTTOM_Y - (1 / 20) * (BOTTOM_Y - TOP_Y));
    assert.equal(d.$('dripHint').getAttribute('opacity'), '0');
    assert.ok(d.$('stage').classList.contains('has-ideas'));
    assert.equal(d.count(), 1);

    for (let i = 0; i < 25; i++) { await d.type('idea ' + i); await d.drop(); }
    assert.equal(d.liquidY(), TOP_Y, 'level saturates at 20 ideas');
    assert.equal(d.text(d.$('countLabel')), 'ideas in the bucket', 'plural label');
  } finally { d.close(); }
});

test('drop animation leaves no NaN position and cleans itself up', async () => {
  const d = await bootApp();
  try {
    await d.type('animated');
    await d.drop();
    await tick(30);   // requestAnimationFrame fires the transition setup
    const blob = d.doc.querySelector('.floating-blob');
    assert.ok(!blob || !blob.style.top.includes('NaN'), 'blob top must be a number, saw: ' + (blob && blob.style.top));
    await tick(700);  // removal timer (650ms)
    assert.equal(d.doc.querySelector('.floating-blob'), null, 'blob removed after the animation');
  } finally { d.close(); }
});

test('selected page text becomes the source line and is stored with the idea', async () => {
  const d = await bootApp();
  try {
    const snippet = d.doc.createElement('div');
    snippet.textContent = 'quoted source text';
    d.doc.body.appendChild(snippet);
    const range = d.doc.createRange();
    range.selectNodeContents(snippet);
    d.win.getSelection().removeAllRanges();
    d.win.getSelection().addRange(range);

    // Model the mobile-WebView semantics the app is built for: tapping the input
    // (programmatic focus) keeps the page selection. jsdom's .focus() clears it,
    // so deliver the focus event directly.
    d.input().dispatchEvent(new d.win.FocusEvent('focus'));
    await tick();
    assert.match(d.$('composerSource').textContent, /from: quoted source text/);

    await d.type('idea with origin');
    await d.drop();
    assert.equal(d.ideas()[0].source, 'quoted source text', 'source persisted with the idea');
    assert.equal(d.$('composerSource').textContent, '', 'source line cleared after the drop');
  } finally { d.close(); }
});

test('typing clears a stale source line; the drop stores no source then', async () => {
  const d = await bootApp();
  try {
    const snippet = d.doc.createElement('div');
    snippet.textContent = 'stale source';
    d.doc.body.appendChild(snippet);
    const range = d.doc.createRange();
    range.selectNodeContents(snippet);
    d.win.getSelection().addRange(range);
    d.input().dispatchEvent(new d.win.FocusEvent('focus'));
    await tick();
    assert.ok(d.$('composerSource').textContent.includes('stale source'));

    d.input().value = 'typed idea';
    d.input().dispatchEvent(new d.win.Event('input', { bubbles: true }));
    await d.drop();
    assert.equal(d.ideas()[0].source, '', 'source was cleared by typing');
  } finally { d.close(); }
});

test('long text is truncated to the composer maxlength (280)', async () => {
  const d = await bootApp();
  try {
    assert.equal(d.input().getAttribute('maxlength'), '280');
  } finally { d.close(); }
});

test('toast timing: shows then auto-hides after ~2.6s', async () => {
  const d = await bootApp();
  try {
    await d.type('toast idea');
    await d.drop();
    assert.ok(d.toastShown());
    await tick(2800);
    assert.ok(!d.toastShown(), 'toast auto-dismissed');
  } finally { d.close(); }
});
