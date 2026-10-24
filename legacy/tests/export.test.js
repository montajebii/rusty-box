'use strict';
// export.test.js — JSON and Markdown exports in a plain browser (no Capacitor):
// the download path must produce a correctly-shaped payload.
const test = require('node:test');
const assert = require('node:assert/strict');

const { bootApp, tick } = require('./helpers/boot');

const IDEA = (over = {}) => ({
  id: 'seed-1', text: 'seeded idea', color: '#f0b429',
  created: 1_700_000_000_000, lastViewed: 1_700_000_000_000, buckets: [], ...over,
});

// 2023-11-14 in UTC; toLocaleDateString in node/jsdom defaults to en-US → 11/14/2023
const DATE_STR = new Date(1_700_000_000_000).toLocaleDateString();

test('JSON export downloads a parseable snapshot of every idea', async () => {
  const d = await bootApp({
    seed: [IDEA({ id: 'a', buckets: ['Work'] }), IDEA({ id: 'b', text: 'two' })],
  });
  try {
    d.$('exportJsonBtn').click();
    await tick(10);
    assert.equal(d.hooks.downloads.length, 1);
    assert.equal(d.hooks.downloads[0].download, 'idea-bucket.json');
    assert.equal(d.hooks.blobs.length, 1);
    const payload = JSON.parse(await d.hooks.blobs[0].text());
    assert.equal(payload.length, 2);
    assert.deepEqual(payload.map((i) => i.id), ['a', 'b']);
    assert.match(d.toastText(), /Exported idea-bucket\.json/);
  } finally { d.close(); }
});

test('Markdown export: one section per bucket, unfiled section last', async () => {
  const d = await bootApp({
    seed: [
      IDEA({ id: 'a', text: 'alpha idea', buckets: ['Work', 'Home'] }),
      IDEA({ id: 'b', text: 'beta idea', buckets: ['Work'] }),
      IDEA({ id: 'c', text: 'gamma idea' }),
    ],
  });
  try {
    d.$('exportMdBtn').click();
    await tick(10);
    assert.equal(d.hooks.downloads[0].download, 'idea-bucket.md');
    const md = await d.hooks.blobs[0].text();
    const lines = md.split('\n');

    assert.equal(lines[0], '# The Idea Bucket');
    // Work has 2 members, Home 1, Unfiled 1 — sections in bucket-index order (count desc)
    const sectionIdx = (title) => lines.findIndex((l) => l.startsWith('## ' + title));
    const work = sectionIdx('Work (2)');
    const home = sectionIdx('Home (1)');
    const unfiled = sectionIdx('Unfiled (1)');
    assert.ok(work !== -1 && home !== -1 && unfiled !== -1, 'all three sections present');
    assert.ok(work < home && home < unfiled, 'sections ordered by count then unfiled last');

    // multi-bucket ideas appear under each bucket with an "also in" note
    const workSection = lines.slice(work, home).join('\n');
    assert.match(workSection, /alpha idea/);
    assert.match(workSection, /_\(also in Home\)_/);
    assert.match(workSection, /beta idea/);
    const homeSection = lines.slice(home, unfiled).join('\n');
    assert.match(homeSection, /alpha idea/);
    assert.match(homeSection, /_\(also in Work\)_/);
    assert.ok(!homeSection.includes('gamma idea'),
      'gamma is unfiled → it must not appear under Home');
    const unfiledSection = lines.slice(unfiled).join('\n');
    assert.match(unfiledSection, /gamma idea/, 'unfiled ideas land in their own section');

    // list items carry a bold date and the idea text
    assert.match(md, /- \*\*.+\*\* — alpha idea/);
    assert.ok(md.includes(DATE_STR), 'uses the locale date of the created timestamp');
  } finally { d.close(); }
});

test('Markdown export with no buckets at all falls back to a flat list', async () => {
  const d = await bootApp({ seed: [IDEA({ text: 'solo thought' }), IDEA({ id: 'b', text: 'another' })] });
  try {
    d.$('exportMdBtn').click();
    await tick(10);
    const md = await d.hooks.blobs[0].text();
    assert.match(md, /## Ideas \(2\)/);
    assert.match(md, /solo thought/);
    assert.match(md, /another/);
  } finally { d.close(); }
});

test('Markdown export of an empty bucket renders without crashing', async () => {
  const d = await bootApp();
  try {
    d.$('exportMdBtn').click();
    await tick(10);
    const md = await d.hooks.blobs[0].text();
    assert.equal(md, '# The Idea Bucket\n\n## Ideas (0)\n');
  } finally { d.close(); }
});
