// A port squatter can pre-fill the browser cache before our process starts.
// SRI in the freshly served index authenticates every executable asset.
// Keep the graph closed: an unreferenced JS chunk would import without SRI.
//
// [Decision Log] Chunks loaded later (milestone 11, ADR 0017's amendment)
// - Purpose: the page is divided by screen (src/app.tsx, src/main.tsx); a
//   screen's chunk is imported when it opens, not named by a tag in the
//   index, so a tag's `integrity` cannot pin it.
// - Choice: an import map in the index whose `integrity` section names
//   every such chunk with its SHA-384. The browser checks any module it
//   fetches from a URL the map names, `import()` included, and refuses a
//   mismatch as it does for a tag. The map is the one inline script the
//   CSP allows, by its SHA-256 (vessel-web's build reads it from this
//   index, `guard` puts it in `script-src`).
// - Alternatives: `<link rel="modulepreload" integrity>` for each chunk
//   (pins them, but fetches every chunk at load, which is what dividing
//   avoids); loading chunks with fetch() and checking their hash in
//   script (needs `blob:` in `script-src`).
// - Still closed: every script and stylesheet in assets/ is named by a
//   tag of the index or, for a script, by the map; a stylesheet must be
//   the index's (vite.config.ts keeps one), else the build fails.
// - Needs a browser that applies import-map integrity: Chrome and Edge
//   127, Firefox 138 (ADR 0017's amendment).
import { createHash } from "node:crypto";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = new URL("../dist/", import.meta.url);
const index = new URL("index.html", root);
/** `path` (from `/`)'s SHA-384 as an SRI value.
 * @param {string} path */
function sri(path) {
  const digest = createHash("sha384")
    .update(readFileSync(new URL(path.slice(1), root)))
    .digest("base64");
  return `sha384-${digest}`;
}
const referenced = new Set();
let html = readFileSync(index, "utf8").replace(
  /<(script|link)\b[^>]*>/g,
  (tag) => {
    if (
      !tag.startsWith("<script") &&
      !/rel="(?:stylesheet|modulepreload)"/.test(tag)
    ) {
      return tag;
    }
    const path = /(?:src|href)="(\/assets\/[^"?#]+)"/.exec(tag)?.[1];
    if (path === undefined) {
      throw new Error("Executable HTML must reference a bundled asset");
    }
    referenced.add(path.slice("/assets/".length));
    return tag.replace(/>$/, ` integrity="${sri(path)}">`);
  },
);
/** @type {Record<string, string>} */
const later = {};
for (const file of readdirSync(new URL("assets/", root))) {
  if (!/\.(?:m?js|css)$/.test(file) || referenced.has(file)) {
    continue;
  }
  if (file.endsWith(".css")) {
    throw new Error(`Asset ${join("assets", file)} is outside the SRI graph`);
  }
  later[`/assets/${file}`] = sri(`/assets/${file}`);
}
if (Object.keys(later).length > 0) {
  // Before any module script: a map after one is ignored.
  const map = `<script type="importmap">${JSON.stringify({ integrity: later })}</script>`;
  const first = html.search(/<script\b/);
  if (first < 0) {
    throw new Error("The index has no script to load the chunks");
  }
  html = `${html.slice(0, first)}${map}\n    ${html.slice(first)}`;
}
writeFileSync(index, html);
