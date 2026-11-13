# Settings track resume boundary

The credit watcher stopped new work on 2026-11-13. Finish only the input-profile
slice already in progress, commit its exact validation status, and stop owned
processes. Start later slices only after the coordinator resumes this track.

## Committed baseline

Branch: `backend/m12-app-settings`. The source checkout is this worktree;
build output remains in its two ignored Cargo configurations. Never commit
those configurations or run the font downloader for the already supplied
Light faces. No push was made.

- `36b9139` and `785bafaf`: machine-book file/view/link actor boundary. V1 passed
  this scope and forwarded it to the coordinator. It is not a Machines UI pass.
  The final native gate passed 385 tests. Typed file failures cover every read
  route, browser edits preserve the Edited response shape, failed edits retain
  selection/clients/follow, rename preserves the real client and watched task,
  and forget releases only that machine without remote stop/delete.
- `1cda42e`: coordinator-requested one-line conversation content-width seam in
  `app/src/areas/session.rs`; C was notified. It accepts future custom points
  through `ContentWidth::points`, preserving the existing native default820.
- `5e3a53c`: integrate the lead's safe file-fault boundary.
- `2f66a6bb`: integrate lead `d050e7e9`; whole native421+2 doc tests passed.
- `5521b30a`: integrate lead `74dfde8e`; whole native429+2 doc tests passed.
  This imports protocol12 and the Light font pair. No independent root-workspace
  validation claim is made for lead-authored root changes.

The inherited uncommitted Machines UI was rejected because it appended detail
under the list and added local control styles. Its complete patch/new files
were saved outside the repository under the track scratch `inherited/`; no
stash was used. It was not committed as a screen.

## Current input-profile slice

Committed `0c9aa6f6` (`kit: settings numeric and rename input profiles`).
Submitted to V1 FIRST; source/UI review is pending, no merge approval assumed.
Final whole native gate exited0 for fmt, Clippy and test:435 tests plus2 docs.
Evidence: track scratch `input-profiles-gate-5/`, including actual kit/app
recompilation logs, cleanup evidence and exact ordered source manifest:
1358 selected files, SHA-256
`6a4ddc4d648558a5f108cc70fc36a992ca54b911a9696801718352aede54b57c`.
Six new GPUI regressions passed. No current source is left uncommitted.

This is control-only. `NumberField` reuses the registered `TextField` line
engine; typed `NumberWidth` chooses64/80, min36 on both densities, py8/px12/r6,
border1/surface2/right aligned engine text. Focus selects all; Enter/blur report
uninterpreted drafts. Parsing/clamping/external preference resync are future
view/app work. `TextField::rename_element` is a separate surface0/p12/r6/border1
profile, with typed `Editability` and `select_all`. New profiles forward a
padding press to that same input engine, respecting consumed/disabled/read-only
and touch-keyboard eligibility. Existing line/composer/multiline entries retain
their geometry and handlers. New fields still enter the shared composition
registry via ordinary `TextField::new`.

Sources: Paseo `screens/settings/appearance/appearance-section.tsx` FontSizeRow,
sizeInput/widthInput, and `components/rename-modal.tsx`. Engine/layout tests are
not live OS IME proof or consuming-page visual acceptance. No page, numeric
preference model, new palette, screenshot or real agent interaction is added.

Input gate history matters: the first run found eight strict semicolon lints;
they were fixed without weakening a lint. Gate2 passed fmt/Clippy but exposed
1418 rather than18 in a selection probe. Gate3 retained the replacement check
and added actual pointer entry; its explicit engine Focus-event assertions
also failed. GPUI suppresses focus events for inactive test windows, which are
the default. Gate4 explicitly activates windows, retains all original behavior
assertions, and covers center/left/right padding of both numeric profiles plus
rename padding/read-only. Do not claim that initial paint alone fixed it, or
that a production engine Focus event was proven broken.

## Review boundary

V1 owns review. Source clearance, a passing gate and actual visual approval
are different claims. No new settings page has captures or UI approval. Every
consuming page still needs actual inspected EN/KO, dark/light,1280/390 captures,
including long/empty/loading/failed/input states. Owned fake fleet only; never
touch the real preview or its services. All own runtime processes must be stopped
by verified PID. Before each compiling/linking cargo command use the machine
cargo-slot helper, one command at a time even while queued. Install prepared
source with current timestamps and verify changed crates recompiled in logs;
never preserve old source mtimes or use cargo clean to work around them.

## Reserved controls not implemented

Do not assume any of these exists because it was discussed.

1. E2 dot profiles: V1 reserved the NEXT separate control-only slice after
   inputs. Proposed `dot_at(color, DotSize::{Inline,Status})` six/eight points,
   compatibility `dot()`/`Leading::Dot` remain six, additive
   `Leading::SizedDot { color,size }` uses one centered14-point option slot.
   Use semantic success/warning/error foregrounds, not board lane colors.
   Preserve G1 SmallGlyph and K AgentMark variants when merging. G3 needs typed8
   for ProviderRow; E2 settings trigger/option first-use remains pending.
2. E2 theme tile/preview: V1 reserved a separate control-only slice after dot8.
   Source `appearance/theme-picker.tsx` and both PressHighlight implementations
   were read. Current area-local tile incorrectly uses label14; source is12.
   Tile r8/p4/gap4/border2, accent selected border, surface2 hover only when not
   selected; no invented pressed paint. Frame r6/border1/clip. Preview16:9,
   sidebar36 and original line/bubble geometry. System clips the right half of
   a full-width dark preview shifted200%/-100% over full-width light; do not
   shrink two previews independently. Proposed Single(&Tokens)/Split(left,right)
   keeps K's theme/list/token ownership; app owns50/25-percent grid and callbacks.
3. G2 owns Gauge for Usage navigation; V1 reserved original24-grid/1.5 glyph,
   pointer16/touch20. G2 stop-boundary report says only prepared externally,
   uncompiled/uncommitted; NO hash this credit window. No substitute icon/character.

## Dependencies, exact status at closeout

- K job2 themes/ADR0049 HAS NOT STARTED. Existing `view::prefs::Theme`,
  appearance.theme, Theme::ALL and PreferenceChange::Theme remain System/Light/
  Dark. Do not duplicate the future eight-theme list or token sets. K is closing
  only its current optical/fitted mark control slice. Its future APIs/hashes
  require source disposition before reuse.
- G2 source-cleared Ghost `1698aab2`, static Rename `dcf48666`, Stethoscope
  `22918fa8`, sized buttons `7006bfb5`; UI/input first-use still pending.
  Ghost is ButtonKind::Ghost. Rename API host_rename_button(id,ButtonState,tokens),
  visual22/hit38/Pencil14/p4/r6/static muted BOTH layouts; no extra density,
  hover, pressed, pending opacity. Caller installs handler only Enabled.
  Sized API button_with_kind_at(...,ControlSizing::{Pointer,Touch},tokens).
  G2 resume note is `docs/plans/backend-m12-g2-resume.md` at `a02b5faa`.
  Sized commit alone requires the shared menu chain; do not import an unknown
  branch just for Reset. G2 clean-lead integration `7f56176c` has whole471+2doc
  gate but its integration disposition was pending; it also contains schedules
  view work. Prefer approved controls through the lead. Do not blindly cherry-pick
  a merged-tree compatibility fix onto a different baseline.
- G3 section_with_trailing/row_with_content `70bcfd5` source-cleared, first-use
  pending. Preserve lead code_row -> row_faced -> row_with_content single renderer.
  Generic app DetailHeader landed in G3 `0f46b7d8` at closeout, whole492+2doc
  gate passed and submitted to V1 FIRST; source/UI review remains PENDING. It
  delegates to G2 screen_header renderer; never build another header. API
  page::DetailHeader { title,back:Rc<dyn Fn(&mut Shell,&mut Window,&mut Context<Shell>)> }
  and page::layout with scroll_id and Option<DetailHeader> before tokens;
  existing ordinary callers pass None. G3 resume `67892a4c` is in
  `docs/plans/backend-m12-agents-resume.md`. Do not consume as accepted yet.
  G3 `b81b7cf8` only fixes two
  canonical TOUCH_TARGET paths and adds its AgentsHub to the machine test State
  fixture on its merged baseline; it is not standalone without that hub.
- G4 Diagnostics collection/report ADR0050 has NO reviewed callable API yet.
  G4 view owns correlated tickets/run state, stale response rejection, partial
  errors, app header aggregation and one English copy artifact. E2 supplies app
  version/platform/time/saved count/identities/availability and only draws/wires.
  Do not invent a collector/report/placeholder request or copy raw logs.
- G4 Devices/Sync stay theirs; import through lead. Future State test fixture
  constructor must preserve sync_calls Hub::default beside device_calls and
  G3 agents Hub::default when those fields land.
- N2 Usage is independent usage.rs. Agreed render(&Shell,&Tokens,bool,&mutContext)
  ->AnyElement. Frame reserves Usage after Models before Sync in host group.
  N2 owns collection/data and explicit opt-in vendor lookup. No landed page/API
  hash received. Future one-renderer registry should drive both drawn/content,
  so its registration is Section::Usage=>Some(usage::render), plus normal module/
  state/event seams. Current actual registry has not been changed.
- L owns language module/own picker/overlay: one FIRST General child call plus
  mod/SettingsState/prefer/start/render seams. Do not touch its preference type,
  row or language behavior. Existing Fluent foundation is already in lead.
  L closeout `8da03f180dc888e3b41d8087c311413e0e44a5bb` preserves that seam as WIP,
  NOT merge-approved: General first child general.rs:110; settings mod/state/
  prefer/start/render plus prefs enum/store fixture seams. L's chooser remains
  System/English/Korean, with captures reported; final build/test/wasm and one
  short pairing-hint recapture remain pending. Resume is in
  `docs/plans/backend-m12-i18n.md`; foreign-line manifest is in L scratch.
- S2 owns Shortcuts and Notifications. Shortcuts settings file is separate from
  its listener, same render contract, minimal frame registration. Diagnostics
  audio test is VOICE playback, not OS notification sound; unsupported row stays
  hidden. S2 has no voice playback API to expose here.

## Machine alias integration still owed before a screen pass

Reuse ONLY existing `view::machine_book::Edit/Name`, not a second BookEdit.
Call owner hooks only after a successful durable edit; failures call none.
E2 must update cached Fleet/card names, Settings detail/back state, and local
name-keyed prefs BEFORE owner callbacks/slow refresh can reuse old aliases.
Stable NodeId-keyed agent defaults do not rename. Remove helper dead_code
expectations when their first caller lands. No remote stop/delete on forget.

- Lead `c8559f34`: session_machine_renamed(from,to), session_machine_forgotten(name),
  draft_machine_renamed(from,to), draft_machine_forgotten(name), all &str/no cx.
  Forget routes an open session back to new chat. Await approved lead integration.
- C prior `87bd6373`, now included in C `9ac5842` being merged by lead:
  conversation_machine_renamed(from,to,cx), conversation_machine_forgotten(name,cx).
  Covers visible ConversationArea only, not session/draft/inbox/side question.
- F `212ed779`, whole427 gate, source disposition pending: app
  workspace_book_applied(&Edit,cx), link Hub.book_applied(&Edit). Two success-only
  calls are included in E2 book.rs/native.rs on F's branch. Keep link call directly
  after durable write; move app call after E2 local alias cache updates. Clone
  follows are cancelled and clients dropped, then renamed projects reread. Already
  dispatched CreateSession cancellation is NOT guaranteed; no remote stop/delete.
- N1 node-only WIP `47a4bcb1`, resume `146eee00` in
  `docs/plans/backend-m12-inbox-resume.md`: final root Clippy passed, exact final
  workspace test was still queued/running at closeout, web theme-paint failure
  unresolved. Native drafts remain external/uncompiled; this is NOT a native
  hook/page registration or merge-approved checkpoint.
- N1 inbox_book_edited(&Edit,cx) RESERVED, NOT LANDED this credit window. It moves
  its own canonical routes/card source, invalidates old tickets, preserves action
  confirmation identity and ignores late replies. N1 consumes fleet only from
  actual Event::Fleet, not old cached fleet during arrive/Acted/manual refresh.
- N5 side-question post-success hook RESERVED, exact name/hash NOT LANDED. It
  must move conversation/request target and reject old late replies; forget clears
  local state only. Do not add calls to missing methods.

Default selection is not yet implemented: current refresh drops default_node.
Lead approved additive Fleet.preferred_machine:Option<Name>, from the SAME book
read as entries. Order current -> remembered -> book default -> first connected;
never overwrite the first two. In settle_draft change only the one helper call
and add an E2 seam comment. Name needs validated serde. Preserve all new Fleet/
Card fields from S2. Shared preferred_name core should serve existing machine_meant
and fleet; do not duplicate policy. Add non-first-default actor regression.

## Next pages and known differences

The ACTUAL nav still has four inherited groups and width264. It has NOT been
fixed or visually approved. Prepared frame collapses to App then one Machines
(host) group; machine picker heads that group, including Machines/Projects/
Devices/Agents/Models/Usage/Sync. App: General/Appearance/Chat/Shortcuts/
Notifications/Diagnostics/About; unsupported reference sections stay hidden.
Desktop sidebar320/surfaceSidebar/right border, pointer16/touch20 section glyphs,
status dot8 and option slot14. Host picker is searchable=false in this consumer.
Use G3 header slot and approved G2 renderer, not another CompactHeader.

General must use source Sending order Interrupt/Steer/Queue, no send-shortcut
hint, and a default-send hint depending on SendKey. Lead confirmed alternate
Mod+Enter queue/submit inversion has NO API; describe actual default behavior
only, record difference, leave future implementation to lead. No supported
native first-run wizard/autostart row exists; do not draw no-op controls.
Preference save failures currently only log; carry a safe typed result to a
localized visible settings error while leaving File state retryable. Refused
validation must be stored typed, not an English Display string.

Appearance: validated Width600..4000 as Custom while named presets round-trip
unchanged. No three-preset UI; only number+px+Reset. UI size10..21/default14,
code9..22/default12 per source. Invalid/empty restores old, floor/clamp on blur/
Enter, font drafts strip ASCII nondigits, external numeric change replaces once.
Legacy native Comfortable paints820, Wide1152, Full is unbounded; a separate
AvailableWidth represents Full below600 without rewriting it on focus/blur.
Reset stores the default marker even Custom820 and explicitly restores the field
because a native div button need not blur first. Numeric rows use raw nonwrapping
shared row profile BOTH layouts (source FontSizeRow/ContentWidthRow do not use
SettingsSelect compactRow). Words wrap beside the fixed control rail.

Coordinator approved small Chat file after Appearance before Shortcuts. Move C's
existing ConversationDisplay row/model/store unchanged. Source chat/chat-section.tsx.
Missing reasoning default, outline visibility, whole-turn folding were sent to C
in one message and added to its list; omit those rows and record each limitation.

Machines must be separate list/detail pages, not appended content. Name validation
trims as HostRenameButton then follows native ASCII64 CLI grammar; pending refuses
edits/cancel/repeat submit, failed save retains draft, success alone moves aliases.
Use G2 static Rename and G3 section/header seams. Keep only reference-backed
parts; no invented extra live-count/Agents panels. Native default action is an
explicit extension. File errors use safe types and one error signal at the proper
location. Pairing-link add already exists; link to it, do not duplicate it.

Diagnostics page waits for G4's typed model/API. Existing About already has private
logs() copy-path/open-folder behavior. Before touching About/Updates prepare a
concrete move/extraction patch then ask coordinator as the user explicitly required;
do not duplicate logs. Voice audio test stays hidden. Source app-diagnostic-sheet.tsx
for progress/copy/refresh; safe copied artifact stays English independent of UI lang.

## Scratch and process discipline

Track scratch is `/tmp/work/m12-settings`. `NEXT.md` is a chronological working
log; this document states the boundary. `next/` contains prepared Rust/scripts,
NONE applied except the current input profile files described above. These are
not validated or commits: numbers/Width/tests/app-numbers, frame/controller split,
General/kept text, Chat, Page/RenameDraft/RenameForm/tests, rekey, fleet/default
fixtures and default actor test. `apply-dot.py` has NOT been run. Never blindly
reapply a scratch script over newer source. Review current APIs and source again.
Install with current mtimes, not copy2/cp-p/tar-preserve.

`gate.py` runs native fmt, slotted whole Clippy, slotted whole tests sequentially,
freezes selected .rs/.toml/.lock source and caches passed identical gates. Never
edit source while it runs. `source-manifest.py` records exact sorted Path order;
string-sort order is different and caused a withdrawn V1 fingerprint finding.
Gate logs and exact ordered manifests are in scratch per-slice folders.

`ui.py` is only a prepared owned-Xvfb/fresh-HOME capture helper; not run. Never
kill inherited old fleet PID files. Start a new owned fleet root and record exact
PIDs. No real preview/services or distribution build output were touched.

Closeout process check: no E2-started fake fleet, app or display existed; the
build gate finished and its descendants exited. A live inspection found the
predecessor E fleet still running since09:52. The exact scratch data directories,
binary paths, PID-file membership, process start ticks and descendants were
verified before signalling. Both fake nodes and six fake adapters were stopped
by individual PID; no matching live process remains. The stale active PID file
was archived to cleanup evidence then removed, preventing a future reused-PID
stop. No real preview/service or another track's process was signalled. Own UI
PID record is absent. This ran as uid1000 and changed no permissions. Cleanup
JSON and the verified process manifest remain in the final gate evidence folder.


Late node handoff, resume only: A2 `a3e78dfa` supplies protocol12 node.settings
(read) and node.set_settings (admin) with typed ParkAfter Never/Idle1..10080/
Unknown. Source `screens/settings/idle-runtime-cleanup-card.tsx` belongs to host
Agents, therefore G3, not E2 General. It is one release-idle-agents switch:
onIdle60 (or its existing minutes), offNever; unknown is Custom and never sent.
Read on show, hide unsupported/invalid_request, show forbidden below the row,
use returned settings. No E2 code, new control or registration was added for it;
G3 was notified for resume. Verify actual reviewed node hash/API before drawing.
