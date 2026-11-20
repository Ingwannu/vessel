# Plan: Paseo's file viewers in the Files tab, page and native (F-F38, F-F39, F-F40; live watch rows)

Status: ruled by the coordinator (2026-11-19); the page side is built
(below, "Page"); native follows on the app line.

## Paseo (pinned 1c00b365c, 0.15.4, `packages/app/src/file-panels/`)

- `catalog.ts`: built-in panels in resolver order: Markdown (`md`,
  `markdown`, text), HTML (`html`, `htm`, text), Image (`png`, `jpg`,
  `jpeg`, `gif`, `webp`, `svg`, image), then Source for every other text
  file. PDF is not built in: a plugin panel shows it in a document frame
  (the browser's PDF viewer).
- `builtin.tsx`: Markdown and HTML are the text panel with a Preview;
  Image is `ZoomableImage` over the file's URL, "Loading" while it
  loads, the error's words in the destructive colour when it fails.
- `source/panel.tsx` and `toolbar.tsx`: a panel with a Preview has a
  Preview|Source control in its toolbar; it opens on Preview, or on
  Source when opened at a line.
- `markdown-preview/`: the front matter (`---` YAML at the top, core
  schema) as a two-column key/value table above the body; the body is
  the chat's Markdown renderer; relative images and file links resolve
  from the Markdown file's folder; a reading frame with a gutter;
  "Loading" until the first layout.
- `html-preview.web.tsx` and `html-preview-csp.ts`: the text in a
  `srcdoc` iframe sandboxed `allow-scripts` only (opaque origin); a CSP meta put first in
  `<head>` allows inline script and style and `data:`/`blob:` images and
  refuses fetch, XHR, sockets, remote script, font and image, forms and
  frames.
- The open file follows the disk (`file/handle.ts`, `setObserving`):
  the preview re-renders from the new text; an image's URL changes with
  the file's version.

## Rows: what exists today

| Row | Paseo | Node (protocol) | Page today | Native today |
|---|---|---|---|---|
| F-F38 Markdown preview + front matter | built in | `file.read` (10) | none: Source only; `react-markdown` renders chat (raw HTML off) | none: Source only; the conversation has a Markdown renderer |
| F-F39 HTML preview | built in, sandboxed frame + CSP | `file.read` (10) | none: Source only | none: Source only; no web engine in the app |
| F-F40 Image viewer | built in, zoomable | `file.read_bytes` (27): 256 KiB pages bound to the first page's revision, files ≤ 64 MiB | none: "Binary file preview is not available." | none: binary note; the composer's lightbox zooms and pans an image |
| PDF | plugin only | `file.read_bytes` (27) | none | none |
| F-F8, F-F29 live file | `fs.file.subscribe/update` | `file.observe` (27), binary too | exists: `use-disk.ts`, one observation at a time, re-read on change, conflict alert while editing | exists: `watch.rs`, same rule |
| F-F3 binary transfer | binary frames | `file.read_bytes` pages (27), `attachment.*` (10) | exists: Download (`download.ts`), uploads (`data/upload.ts`) | exists: `download.rs`, composer attachments |
| F-F6 download | token + HTTP | `file.read_bytes` (27) | exists: Download in the Explorer menu | exists: Download in the Explorer menu |
| F-F7 upload | composer attachments | `attachment.*` (10) | exists | exists |
| F-F15, F-F44 directory observer | server watcher pushes git changes | none (only per-file `file.observe`) | Changes polls `git.changes` every 5 s while visible; the tree refreshes on its Refresh, as Paseo's | Changes polls; tree Refresh |
| F-F45 directory sync | sequenced replica of agents, workspaces, projects | session/project events and lists | not a file feature; lists follow events | same |
| F-F17, F-F46 folder suggestions, project icon | RPCs | `folder.suggest`, `project.icon` (32) | done (merged) | done (merged, f8928581) |

So the missing whole features are the three viewers. The live-watch
rows exist on both sides; what is missing is that the new viewers follow
the disk too (the image's bytes re-read on a new revision), which the
viewers take from the existing watch. No node change is needed.

## Steps

1. Page routing: the Files tab picks the panel by extension in Paseo's
   order (Markdown, HTML, Image, else Source); Markdown and HTML get
   Preview|Source in the file head (kit segmented control), opening on
   Preview, or Source when opened at a line (search hit, go-to-line).
   Editing stays Source's.
2. Page Markdown: front matter table (Paseo's parser: the `yaml`
   package, core schema, values stringified as Paseo's
   `document.ts`), body through the page's Markdown renderer, relative
   images read with `file.read_bytes` and shown as `data:` URLs,
   relative links open the file in the Files tab, absolute web links as
   in chat.
3. Page image: the pages joined, shown as a `data:` URL, Paseo's zoom
   (wheel and pinch zoom, drag to pan, fit on open), "Loading" and the
   error's words; re-read when the watch reports a new revision.
4. Page HTML: Paseo's `srcdoc` frame (`sandbox="allow-scripts"`) with Paseo's CSP meta
   inserted as `html-preview-csp.ts` does (ported with its tests).
5. Native: the same routing and Preview|Source; Markdown through the
   conversation's renderer plus the front matter table; images with
   gpui's decoders and the lightbox's zoom and pan; follows the disk
   through `watch.rs`.
6. Tests per step (unit for routing, front matter, CSP insertion, page
   joining; e2e on the fake fleet for each viewer), captures for the
   coordinator, then the packet notes.

## Differences (proposed)

1. Page HTML preview runs no script. An `about:srcdoc` frame inherits
   the page's own CSP (`crates/vessel-web/src/guard.rs`:
   `script-src 'self'` plus the import map's hash), so inline script in
   the previewed file is refused whatever the frame's meta says; layout,
   inline styles and `data:` images show. Running scripts would need a
   separate preview origin served by vessel-web (an ADR of its own).
   Closed by TRACKER 814 (ADR 0122, 2026-11-20): the page posts the
   document to `vessel ui` and frames it from `/preview/<id>`, whose
   header carries Paseo's policy with `sandbox allow-scripts`; the
   scripts run as Paseo's do. The static `srcdoc` frame stays as the
   fallback when the route cannot be reached. What remains is Paseo's own
   stated hole: a document may navigate itself.
2. Native HTML: Source only. The app has no web engine (ADR 0020, Tauri,
   still waits); the Preview|Source control is absent for HTML there.
3. Images on the page are `data:` URLs (the page CSP allows `data:`, not
   `blob:`); Paseo hands its viewer an HTTP URL. Same bytes, no CSP
   change. A file over the node's 64 MiB bound shows the node's refusal.
4. PDF is not in this item: Paseo ships it only as a plugin. On the page
   it would need `frame-src blob:` (or `object-src`) in the page CSP; on
   native a PDF rasteriser dependency. Offered after parity as an extra.
5. The front matter parser on the page is the `yaml` package (Paseo's
   own dependency, no transitive dependencies); native uses the YAML
   crate the workspace already locks, or a new one if none is locked
   (to be stated in the packet).
6. Changes stays on its 5 s poll (F-F15/F-F44): Vessel's node has no
   directory observer; adding one is a node feature on its own, not
   needed for the viewers.

## Page (built)

- Routing: `web/src/data/workspace/file-panels.ts` (Paseo's catalog and
  resolver: the extension after the base name's last dot, lower case).
  The file view (`features/workspace/file-view.tsx`) draws the panel;
  the head (`file-head.tsx`) holds Preview|Source, opened as Paseo's
  `useTextFileView` opens it.
- Markdown: `data/workspace/front-matter.ts` (the `yaml` package, core
  schema, Paseo's value formatting; Paseo's five tests ported),
  `data/workspace/markdown-paths.ts` (a link or image target from the
  Markdown file's folder, kept under the root), `file-preview.tsx` (the
  table and the body through `data/markdown.tsx`, which takes the
  preview's link and image hooks; the conversation passes none).
- HTML: `data/workspace/html-preview.ts` (Paseo's prologue and policy,
  its tests ported); the frame `sandbox=""` (ruling 1).
- Image: `data/workspace/file-bytes.ts` (the pages of
  `file.read_bytes`, shared with Download, and the `data:` URL),
  `use-image-url.ts`, `file-image.tsx`, `zoomable-image.tsx` and
  `use-viewport-gestures.ts` over `data/viewport.ts` (Paseo's geometry,
  its tests ported). Paseo's image panel sets the wheel to always zoom
  and the minimum scale to 1.
- Live: the file's existing watch (`use-disk.ts`) re-reads the file; the
  Preview draws the new text, the image is read again for the new
  version (`versionKey`). A Preview reads the pages not read yet, since
  it draws the whole file, up to Paseo's web display budget (50 MiB,
  `PREVIEW_LIMIT`); past it the Preview says "This file is too large to
  display", as Paseo's text panel does (node reviewer's note, 2026-11-19).
- Tests: unit tests per data module; `web/e2e/viewers.spec.ts` (each
  panel, Source beside Preview, a line target, a relative link, the
  frame's script running and reaching neither the page, its storage nor
  the network (TRACKER 814), zoom and fit, a new image on disk);
  `web/e2e/code.spec.ts`'s watch test now starts on the Markdown Preview.

## Differences found while building

7. A Markdown link or image outside the workspace root is not followed
   (the node serves files under the root only; Paseo reads any path its
   daemon can), and a web image is shown as a link with its words (the
   page's policy loads images from itself and `data:` only).
8. The page's Markdown renderer has no GitHub extensions (tables, task
   lists, strikethrough): `react-markdown` runs without `remark-gfm`,
   in the conversation too (the M7 plan recommended it; it was not
   added). Paseo draws tables. Adding it is one dependency and changes
   the conversation as well, so it is left to its own item.

## Decision Log

- Purpose: the Files tab shows Markdown, HTML and images as Paseo's
  built-in panels do, on page and native, and they follow the disk.
- Constraint: the node gives text (`file.read`), exact byte pages
  (`file.read_bytes`) and a finite watch (`file.observe`); the page's CSP
  refuses inline script and `blob:` images; the native app has no web
  engine.
- Alternatives: a vessel-web route serving file bytes to an `img`/frame
  (needs a token route and a CSP change; rejected for now: the bridge
  already carries the bytes); a separate preview origin for HTML with
  scripts (deferred to its own ADR).
- Choice: client-only viewers over the activated protocol 27 calls,
  Paseo's routing and words, two stated gaps (HTML script on the page,
  HTML on native).
- Cost: an image is held in memory as base64 on the page (bounded by the
  node's 64 MiB); HTML previews on the page are static.

## Coordinator rulings (2026-11-19)

1. Page HTML: ship the static preview now, a sandboxed `srcdoc` frame
   without scripts under today's page CSP; "scripts do not run" is a
   stated difference. Running scripts as Paseo does (its
   `allow-scripts` frame) needs a separate preview origin: TRACKER 814,
   with its own ADR, after this item. The page's CSP is not loosened.
   (2026-11-20: TRACKER 814 took option A, a preview route on the same
   `vessel ui` server, sandboxed by its own header; ADR 0122.)
2. Native HTML: Source only, a stated difference until ADR 0020 (a web
   engine) is decided.
3. PDF: plugin-only in Paseo, so out of core parity; left for the plugin
   work (area M), not built here.
4. The other differences are accepted. Order: TRACKER 234 through the
   node reviewer, then the native sheet with Paseo's four methods only,
   then these viewers.
