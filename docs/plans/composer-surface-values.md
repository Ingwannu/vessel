# Shared composer surface values

This standalone leaf on packagebase6+dd13 contains the exact surface.rs and surface/tests.rs blobs from B2 checkpoint8403be97. It exposes view::composer_shell::surface::Mode::{Chat,Terminal} without B2's input, commands, Markdown engine or node branch. No dependencies or runtime registration change.

[Decision Log]
- Purpose: W3/W6 workspace hosts need one canonical mode to suppress chat affordances and select the font.
- Existing assets: B2's pure Surface controller already owns mode, access, loading policy and compact/fullscreen transitions.
- Alternatives: another Launch-to-boolean policy or merging B2's entire app tree.
- Choice: retain the byte-identical pure value leaf plus its tests and two module registrations.
- Why: W3 owns Launch semantics (Chat→Chat; BlankTerminal/Profile→Terminal), while the composer consumes Mode. Missing owners remainNone, with no global/cwd fallback.
- Costs/effects: the leaf includes the surface controller because Mode already belongs to that public path; no second type or editor. Current B2 later adds a read-only overlay getter; keep it on union.

Prior-source evidence: exact8403be97 strict kit+view Clippy0 and view23/0, later unchanged surface included in view27/0. This new base combination is STAGED, not newly compiled or a package/runtime pass. Consumers check their own union. Reference composer/input-mode.ts:1-47 and composer/input/input.tsx:1205-1224.
