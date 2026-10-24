'use strict';
// boot.js — load the real index.html into jsdom with script execution enabled,
// plus hooks that let tests stub browser APIs (confirm/prompt/clipboard/downloads)
// and seed localStorage before the app boots.
const fs = require('node:fs');
const path = require('node:path');
const { JSDOM, VirtualConsole } = require('jsdom');

const ROOT = path.resolve(__dirname, '..', '..');
const STORAGE_KEY = 'idea-bucket-v1';

const tick = (ms = 0) => new Promise((r) => setTimeout(r, ms));

async function bootApp({
  url = 'https://localhost/',
  seed = undefined,          // array of ideas to place in localStorage before boot
  seedRaw = undefined,       // exact string to place in localStorage (corrupt-data tests)
  confirmResult = false,     // what window.confirm returns
  promptResult = null,       // what window.prompt returns
} = {}) {
  const hooks = {
    confirmCalls: [],
    promptCalls: [],
    blobs: [],
    downloads: [],
    clipboard: [],
    confirmResult,             // read at call time, so tests can flip mid-test
    promptResult,
  };

  const html = fs.readFileSync(path.join(ROOT, 'index.html'), 'utf8');
  const virtualConsole = new VirtualConsole(); // swallow jsdom "not implemented" noise

  const dom = new JSDOM(html, {
    url,
    runScripts: 'dangerously',
    pretendToBeVisual: true,   // requestAnimationFrame support
    virtualConsole,
    beforeParse(window) {
      window.scrollTo = () => {};
      window.scrollBy = () => {};
      window.confirm = (msg) => { hooks.confirmCalls.push(String(msg)); return hooks.confirmResult; };
      window.prompt = (msg) => { hooks.promptCalls.push(String(msg)); return hooks.promptResult; };
      window.URL.createObjectURL = (blob) => { hooks.blobs.push(blob); return 'blob:stub-' + hooks.blobs.length; };
      window.URL.revokeObjectURL = () => {};
      window.HTMLAnchorElement.prototype.click = function click() {
        hooks.downloads.push({ href: this.getAttribute('href'), download: this.getAttribute('download') });
      };
      Object.defineProperty(window.navigator, 'clipboard', {
        value: { writeText: async (t) => { hooks.clipboard.push(t); } },
        configurable: true,
      });
      if (seedRaw !== undefined) {
        window.localStorage.setItem(STORAGE_KEY, seedRaw);
      } else if (seed !== undefined) {
        window.localStorage.setItem(STORAGE_KEY, JSON.stringify(seed));
      }
    },
  });

  const win = dom.window;
  const doc = win.document;

  const driver = {
    dom, win, doc, hooks,
    $(id) { return doc.getElementById(id); },
    text(el) { return el ? el.textContent : null; },
    // --- storage ---
    raw() { return win.localStorage.getItem(STORAGE_KEY); },
    ideas() { const r = this.raw(); return r ? JSON.parse(r) : []; },
    // --- composer ---
    input() { return this.$('ideaInput'); },
    async type(text) { this.input().value = text; },
    async drop() { this.$('dropBtn').click(); await tick(20); },
    // --- drawer ---
    async openDrawer() { this.$('openBtn').click(); await tick(1); },
    drawerOpen() { return this.$('drawer').classList.contains('open'); },
    cards() { return [...doc.querySelectorAll('#ideaList .idea-card')]; },
    cardTexts() { return this.cards().map((c) => c.querySelector('p').textContent); },
    bucketChips() { return [...doc.querySelectorAll('#bucketBar [data-bucket]')]; },
    bucketChipNames() { return this.bucketChips().map((c) => c.dataset.bucket); },
    composerChips() { return [...doc.querySelectorAll('#composerBuckets [data-bucket]')]; },
    composerChipNames() { return this.composerChips().map((c) => c.dataset.bucket); },
    // --- idea page overlay ---
    ideaPage() { return doc.querySelector('.idea-page'); },
    ideaPageCard() { return doc.querySelector('.idea-page-card'); },
    async openIdeaPage(index = 0) {
      this.cards()[index].querySelector('.view-btn').click();
      await tick(20);
      return this.ideaPageCard();
    },
    async closeIdeaPage() {
      const btn = this.ideaPageCard().querySelector('[data-close]');
      btn.click();
      await tick(320);           // close() removes the overlay after 280ms
    },
    // --- misc ---
    count() { return Number(this.$('count').textContent); },
    toastText() { return this.$('toast').textContent; },
    toastShown() { return this.$('toast').classList.contains('show'); },
    liquidY() { return Number(this.$('liquid').getAttribute('y')); },
    async addBucketViaPrompt(name) {
      this.$('composerBuckets').querySelector('[data-new-bucket]').click();
      // window.prompt stub returns promptResult synchronously; chip list re-renders
      return this.composerChipNames();
    },
    close() { try { win.close(); } catch { /* already closed */ } },
  };

  // wait for the load event (inline scripts already ran synchronously during parse)
  if (doc.readyState !== 'complete') {
    await new Promise((r) => win.addEventListener('load', () => r()));
  }
  return driver;
}

module.exports = { bootApp, tick, STORAGE_KEY, ROOT };
