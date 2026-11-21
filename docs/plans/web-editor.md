# Web source editor and Vim

M12 F-L8 and TRACKER 459. Settings › Editor contains the **Vim keybindings**
switch, between Terminal and Files. Paseo 0.15.4 places it there in
`packages/app/src/screens/settings-screen.tsx:194–200` and renders the switch in
`packages/app/src/screens/settings/editor-section.tsx:9–32`. Vessel's hint
describes source editing on the page because this packet is web-only.

## Decision log

- Purpose. Give source edits standard editor input, undo, search, line numbers
  and optional Vim modes without changing which version is saved.
- Context. `FileView` and `useFile` own the file revision, unsaved draft and
  conflict choices. The UI bridge already persists page preferences across
  launches. Keeping that file as the owner makes the Vim switch work across
  machines. A new or older preference file defaults to ordinary editing.
- Alternatives. A Vim layer on a textarea would own selection, IME, modal
  commands and undo. Loading an editor with the read-only workspace would
  charge all readers for those editing functions.
- Choice. Reuse the FileView inputs and callbacks and lazy-load CodeMirror
  on Edit with a reconfigurable Vim compartment. Ctrl/Cmd+S takes the same
  guarded path as Save. FileView supplies replacement drafts, and those
  transactions stay out of undo history. The initial CRLF separator governs
  new line breaks in a CRLF file. The tested stale-save and Reload paths live
  at the existing file ownership boundary.
- Effects. The editor reports its Vim mode and follows workspace typography.
  Standard editing keeps Tab available to leave the editor. Find uses the
  standard CodeMirror panel. The edit chunk omits language packs, so editing
  uses plain source text and read-only views retain their existing syntax
  highlighting. Paseo references are `file-panels/source/editor/view.web.tsx:63–114,138–158`
  and `file-panels/source/editor/extensions.web.ts:30–45` under `packages/app/src`.

## Dependencies and evidence

The six direct dependencies match the approved Paseo 0.15.4 pins. Each is MIT,
listed in About, and recorded with integrity in `web/package-lock.json`.

| Package | Version |
| --- | --- |
| `@codemirror/commands` | 6.10.4 |
| `@codemirror/language` | 6.12.4 |
| `@codemirror/search` | 6.7.1 |
| `@codemirror/state` | 6.7.1 |
| `@codemirror/view` | 6.43.6 |
| `@replit/codemirror-vim` | 6.3.0 |

The production build measured these assets in kB, minified and gzipped.
The baseline is node `d107c7553`. The editor chunk loads on Edit.

| Asset | Baseline minified / gzip | With the editor minified / gzip |
| --- | ---: | ---: |
| Entry JS | 456.95 / 144.65 | 457.05 / 144.76 |
| Workspace panel JS | 178.79 / 56.59 | 178.78 / 56.57 |
| Lazy editor JS | — | 429.65 / 137.87 |
| Shared CSS | 234.47 / 49.21 | 235.87 / 49.48 |

The browser test drives the saved setting, verifies lazy loading, edits in
Normal and Insert mode and saves a real CRLF fixture. The existing file and
split-pane tests still exercise draft, conflict and dirty-state behavior.
