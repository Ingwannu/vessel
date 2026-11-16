# Native persistence state is platform-scoped

[Decision Log]
- Purpose: keep the browser compilation free of native-only persistence state.
- Existing assets: storage capture/save is already guarded off for wasm; browser
  intake retains its existing in-memory behavior.
- Evidence: A1 receiving App wasm check406 completed rc0 but emitted four B1
  warnings: unused Incoming::stored, Writing variants, LocalState.writing and
  SAVE_DELAY. The raw log, rather than its paraphrase, identifies those symbols.
- Choice: gate those native-only items and Duration import with the same existing
  target condition; do not add a warning suppression, no-op persistence API or
  second store. Native fields/defaults and behavior remain as before.
- Impact: this is a source-only correction until receiving wasm verification.
  It supplies no IndexedDB/browser persistence or wasm interaction proof.
