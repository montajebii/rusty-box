'use strict';
// route.test.js — deep links (/idea/<id>) survive reloads; unknown ids bounce
// back to the drawer with a toast.
const test = require('node:test');
const assert = require('node:assert/strict');

const { bootApp } = require('./helpers/boot');

const IDEA = (over = {}) => ({
  id: 'seed-1', text: 'seeded idea', color: '#f0b429',
  created: 1_700_000_000_000, lastViewed: 1_700_000_000_000, buckets: [], ...over,
});

test('visiting /idea/<id> opens that idea’s page after a reload', async () => {
  const d = await bootApp({ url: 'https://localhost/idea/seed-1', seed: [IDEA()] });
  try {
    assert.ok(d.ideaPage(), 'idea page overlay present after boot');
    assert.match(d.ideaPageCard().querySelector('.idea-page-title').textContent, /seeded idea/);
  } finally { d.close(); }
});

test('visiting /idea/<unknown-id> redirects to / and opens the drawer with a toast', async () => {
  const d = await bootApp({ url: 'https://localhost/idea/gone-forever', seed: [IDEA()] });
  try {
    assert.equal(d.ideaPage(), null, 'no overlay for a missing idea');
    assert.equal(d.win.location.pathname, '/', 'bounced back to the root route');
    assert.ok(d.drawerOpen(), 'drawer opened so the user can pick another idea');
    assert.match(d.toastText(), /isn’t here anymore|isn't here anymore/);
  } finally { d.close(); }
});

test('the root route opens no overlay', async () => {
  const d = await bootApp({ seed: [IDEA()] });
  try {
    assert.equal(d.ideaPage(), null);
    assert.ok(!d.drawerOpen(), 'drawer closed on a normal visit');
  } finally { d.close(); }
});
