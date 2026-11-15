// Own minimal DOM boundary fixture; no browser/network or reference implementation.
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const source = readFileSync(new URL('./traverse.js', import.meta.url), 'utf8');
const textBytes = 80_000; // Actual Rust owner is MAX_PAGE_TEXT_BYTES; fixture mirrors it.
const config = {
  firstRef: '1', roles: [],
  limits: { nodes: 1500, references: 500, depth: 32, textBytes },
};
function traverse(document) {
  return runInNewContext(`(${source})(config)`, { document, config, TextEncoder }, { timeout: 1000 });
}
test('document content is captured once in the same synchronous traversal', () => {
  let titles = 0;
  let addresses = 0;
  const value = traverse({
    body: { firstChild: null },
    get URL() { addresses += 1; return 'https://example.test/own-snapshot'; },
    get title() { titles += 1; return titles === 1 ? 'Observed title' : 'Later title'; },
  });
  assert.equal(addresses, 1);
  assert.equal(titles, 1);
  assert.equal(value.document.address, 'https://example.test/own-snapshot');
  assert.equal(value.document.title, 'Observed title');
  assert.equal(value.nodes[0].name, 'Observed title');
});
test('a display-only non-HTTP address creates no URL or network capability', () => {
  const value = traverse({ body: { firstChild: null }, URL: 'data:own-content', title: 'Own' });
  assert.equal(value.document.address, 'data:own-content');
});
test('an oversized address is refused instead of presenting a shortened destination', () => {
  assert.throws(
    () => traverse({ body: { firstChild: null }, URL: 'x'.repeat(textBytes + 1), title: '' }),
    /snapshot_address_exceeds_budget/,
  );
});
test('title and root-label UTF-8 bytes share the graph budget and mark truncation', () => {
  const value = traverse({ body: { firstChild: null }, URL: 'https://example.test/', title: '한😀'.repeat(textBytes) });
  const bytes = [value.document.address, value.document.title, value.nodes[0].name]
    .reduce((size, text) => size + Buffer.byteLength(text, 'utf8'), 0);
  assert.ok(bytes <= textBytes);
  assert.equal(value.truncated, true);
  assert.ok(!value.document.title.includes('\ufffd'));
});
