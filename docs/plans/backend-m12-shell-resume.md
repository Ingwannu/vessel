# Shell track resume checkpoint

> 2026-11-14 (track S3): the two prepared patches below are installed and
> gated as `0cf43664` (kit correction) and `153da701` (palette consumer);
> see `backend-m12-shell.md`, "Palette finished". What follows is the
> earlier checkpoint, kept as it was written.

Branch: `backend/m12-app-shell`. This session stops after the integration
checkpoint; no next feature slice was started. Read the binding track
briefs again before resuming. The detailed running notes and artifacts
remain in the assigned `m12-shell` scratch directory.

## Committed and verified

- `8cdc7ea6c0bf1b9c556436463b88b011e532e37c`: shared pure
  `view::search::{Query, Score}` extraction. V1 passed its source scope;
  433 tests and two documentation tests passed. This is not UI approval.
- `babeeb16a7c82b982ff001ba05869d3132973cb4`: control-only typed command
  captions and additive shared snap-sheet state/gesture/motion. Native
  fmt/clippy passed; 445 tests and two documentation tests passed. The
  GPUI probe exercises App reduced-motion true, handle expansion, content
  pan without row activation, a working stationary click and pan-down
  close. The app has NOT consumed these APIs: it still calls the static
  compatibility frame and draws plain subtitles.
- `31321054bf65f56b1978994a6d6571497d693f9b`: clean automatic merge of lead
  `b6bdd43ce6894ffe9ba484a142f7d1a0d5de4dce`. Native fmt/clippy passed;
  478 tests and two documentation tests passed, zero failures. The first
  target `64b5a3d0` failed with C's act-handler borrow; the lead's exact
  correction is included. Final logs: `lead-b6bdd-fmt.log`,
  `lead-b6bdd-clippy.log`, `lead-b6bdd-test.log` in scratch. Changed crates
  were actually recompiled. No root/web gate or new UI pass is claimed.

The merge retains reviewed controls, embeds the already provided Light
faces, and imports protocol12 search/parking, machine-book boundaries,
permission-card seams and the motion follower. No font download ran.
Linux/FreeBSD portal following and execution-only
`VESSEL_APP_REDUCE_MOTION=1|0` are present. Actual OS startup/live-change
behavior is unverified; Windows/macOS following is absent. Incoming
canonical generated TypeScript whitespace was preserved.

The native and root Cargo configuration files are local build settings;
keep them and never commit them. Every build command still uses the
shared cargo-slot helper. No push was made.

## Review status and the first correction

V1 has the control hash and gate evidence, but there has been NO final
palette/control UI review request for this new code. V1's old return on
`6d29e29`/`08382d9` is still open: typed path consumption and real snap/
fade behavior need new captures. The previous70 originals and their
manifest prove only those old immutable trees.

Self-review caught an additional mistake before final UI submission:
Paseo `command-center.tsx`945–949 conditionally unmounts its desktop
Modal at close, so only the entering fade runs. `babeeb16` incorrectly
keeps a250ms outgoing desktop fade. V1 was informed. Remove that outgoing
fade; retain the compact sheet's closing spring and backdrop target
(`sheet-backdrop.web.tsx` keeps the target until portal dismissal). Do
not justify the desktop fade as a necessary GPUI difference.

## Prepared only; not installed or compiled

Two immutable scratch patch artifacts pass `git apply --check` against
this checkpoint. This is text applicability, NOT compilation or review.
Inspect them before applying, and preserve current timestamps on source
installation. Never copy prepared Rust with old mtimes or use cargo clean.

1. `resume-motion-correction.patch`, SHA-256
   `3aff8ada9e7a2cfaf1f8c47895328be8da14c65d0fc4b5c6a84dcb80859b575c`.
   Four kit files: `command/state.rs`, `command/frame.rs`,
   `sheet/snap/motion.rs`, `sheet/snap/tests.rs`. It removes desktop exit
   fade, adds computed `CommandState::is_open`, makes the lifetime test
   use an injected time, and waits for resize rather than requesting
   frames at zero viewport height. The compact exit alone retains input
   blocking. It adds one new test; do not reuse445 as its gate count.
2. `resume-palette-consumer.patch`, SHA-256
   `ac0dc4653b92b8c9a0cb7a8e7629090c15a48730b8ab8378c42d56f09e417551`.
   Six view/app files. It needs the corrected control's `is_open` API
   first. A typed status/path caption keeps the WHOLE visible sentence
   as one search field, then projects UTF-8 marks into the prose prefix
   (separator included) and code path. Korean, separator, expanded
   lowercase and score-preservation cases are prepared. The app delegates
   typography to the kit, restores prior focus once (or blurs if none),
   clears close/reopen state, and lets the control own visual lifetime.

The patch manifest is `resume-prepared-manifest.json`. Source counterparts
are `motion-correction/` and `palette-fix/ready/` in scratch. The older
`palette-preview/` and direct `palette-fix/consume.py` version are NOT the
final consumer; do not run that older script on the checkout blindly.

Next work order after authorization resumes: correction-only slice and
whole native gate/commit; separate typed-view/app consumer and gate;
fresh debug app and fake-helper builds; fresh fake fleet; captures and
LOOK at every original; then formal V1 requests with fixed hashes,
reference files, gates, executable build-id and capture manifest.

Capture preparation exists but has not run: `capture-palette-v2.py`,
`capture-palette-parts.py`, `cap-motion.py` (drag and action timing logs),
`palette-fix/font-fixtures.py` (UI14/code18 and UI20/code10). A short
Korean fixture-root name makes actual Korean path text visible at390.
Verify Unicode query injection, stable60→90→60, pan-down close, scrolling
versus clicks, entering fade, immediate desktop close, fast reopen/focus
and reduced-motion1/0. Build against the new protocol12 node helpers.

Numeric reference evidence: `sample-reference-spring.cjs` executes the
actual installed Reanimated helper; `reference-spring-cases.json` and
`reference-spring-source.json` record seven finite samples, source SHA
and configuration. Full reference paths are in
`command-motion-review-draft.md`; its old exit-deviation description
must be replaced by the correction above. Reference versions were
Gorhom5.2.14, Reanimated4.5.5 and RNW0.21.2, all MIT.

## Other tracks and remaining work

- C: `conversation_find` is already in this merge, but private `hear_find`
  is still registered. Connect registry Find, send C the exact caller
  hash, and obtain/remove the old observer before claiming global IME
  suppression. C's new queue entry points are at `4a172b2c`, not consumed:
  steer-next defaults to Mod+Shift+Enter when not terminal-focused and
  must ignore repeats; edit-latest defaults to Alt+ArrowUp only when the
  composer is focused with collapsed caret0. Remove their dead-code
  expectations at first use. C's grip Enter/Escape/arrows are local keys.
- Lead: `TextField::focused_composing` is integrated. The later
  `TextField::at_start(&self, window, cx)` seam is at `240438a6`, NOT
  merged here; it requires focused selection0..0. Composition is checked
  separately. Repeat handling is unresolved: pinned GPUI calls
  interceptors, bindings, then raw capture/bubble. A root raw capture
  alone is too late to stop an already executed interceptor command;
  the lead acknowledged this ordering. Prove any solution with actual
  GPUI events before using it. No core/kit patch or held-key tracker was
  added for repeat handling.
- F: public `Shell::toggle_code(window,cx)` is in `7fc73539`; consume the
  gated tree `212ed779` (427 tests), respecting merge order. It shares the
  panel button, opens fullscreen at phone width and does nothing without
  a session. It is not consumed here. F reports no private global panel
  keys; request additional file/search/navigation entries only if needed.
- E2: Shortcuts is S2's file inside the shared settings frame, registered
  in both `content` and `drawn`; do not add another header. Preserve its
  one book/preferences model and future default-machine seam. Board
  scope and other retained book-name aliases still need rename/forget
  integration. Diagnostics voice Test audio is distinct from OS notice
  sound and has no S2 test-play API yet.
- N1: finishing ONLY the node slice at this checkpoint; NO native gated
  hash is delivered. The proposed Inbox/Pages/Gate/presence APIs and
  enqueue deadlines are prepared, not approved/compiled integration.
  Do not consume mutable work as passed. N1 owns inbox rows/page and
  policy; S2 owns outside navigation/count, waiting lane, OS delivery and
  sound. Head16 per node is not every waiting key; exact waiting count
  counts requests, not sessions. Future waiting-session projection should
  retain node `SessionBoard.activity` in CardDetail with serde default;
  `NeedsYou` also includes PR/check feedback and is not a request proxy.
- K: optical/fitted controls and app captures are not finally passed.
  Theme Job2/ADR0049 has NOT started: Theme/ALL remain System, Light,
  Dark. Do not assume eight themes. Preserve K's small official-mark
  insertion points when a reviewed hash arrives.
- G3: the one shared alphabetical comparator remains G3-owned, prepared
  outside the repository and uncompiled, with no API hash to consume.
  Coordinate before adding or importing it next session; do not duplicate
  it or introduce ICU work. `8cdc7ea6` search is source-cleared, but G3
  reports it was not imported into its unfinished registry slice. The
  coordinator's comparator policy remains Unicode lowercase followed by
  original code-point ties, with locale-tailoring differences explicit.
- G2/V1: there is NO S2 shared NavRow/leading API or reservation. V1 is
  coordinating the existing row owner for CalendarClock in the schedules
  slot. Preserve that seam later. Sidebar menus must reuse G2's single
  engine; no local duplicate menu. Shortcuts' trailing32 rail is page
  layout, not a new menu API. G1 owns the shared Info toast extension.

Board/card/archive/sidebar and the shortcut page/notifications remain
unreviewed. Board card/module files exceed the target size; local ghost
and icon buttons, cancel-button variant, code metrics and tooltip-cache
lifetime need correction through the reviewed kit. Inspect root error
adapters before changing error wording. No new standalone palette,
sidebar, shortcut, notification or sound pass is claimed.

## Cleanup

The old capture fleet was already stopped. Two inherited fake nodes were
found by their recorded PID and matching fixture data path and sent
SIGTERM; both were verified gone, with no remaining fake-acp/fake-tui
process in the owned fixture working directories. Stale PID markers were
removed after verification. X155 is absent. No preview/service was
changed. Work ran as the repository account; no ownership or service
permission change was made. All build commands for this checkpoint ended
before the session stopped.
