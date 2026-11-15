# F4 command center and keyboard parity

Status: implementation in progress on package base 4 (3cd0d3fc), merged as
f12d347b. S2's exact theme/input dependency eddce254 is reviewed V1 PASS.
Canonical defaults, dispatcher, Settings/help and source projection are written;
app compilation, feature-owner integration and package verification remain open.
No command-center or keyboard tracker row is passed. No new node RPC is needed
for this package's mechanics; feature-owned RPC targets remain with their owners.

## Existing assets and choice

**Modify existing assets.** `view::shortcuts` already owns the action registry,
key grammar, matcher, conflict edits and recorder; `app::areas::shortcuts` owns
the sole interceptor and Settings rows. `view::palette`, its shared matcher and
cursor, and `app::areas::palette` already own the command center. Extend these;
never introduce another key listener, palette or independently maintained table.
The standard library suffices for registration ownership and ordered maps. A
third-party shortcut or fuzzy-search engine would introduce incompatible
scoring, native editing and shortcut defaults, so none is added.

## Source behavior read before implementation

Paseo source paths below are relative to packages/app/src.

- `keyboard/keyboard-shortcuts.ts:259-1176`: 76 default binding variants,
  per-platform predicates, scopes, repeat and propagation policy. Historical
  binding ids are persistence ids even when the current combo differs.
- `keyboard/keyboard-shortcuts.ts:1228-1275,1386-1594`: overrides replace combos;
  chords wait 1500 ms and candidate sets narrow; initial chord prefixes take
  precedence over single-key matches; contexts are rechecked at every step.
- `keyboard/shortcut-string.ts:27-179`: logical characters first, shifted
  punctuation and non-Latin/Option physical fallbacks, digit wildcards.
- `keyboard/keyboard-action-dispatcher.ts:1-220`: scoped action payloads and
  priority, then newest-registration-first handlers; stale cleanup cannot
  remove a replacement registration; unhandled actions fall through.
- `hooks/use-keyboard-shortcuts.ts`, `keyboard/focus-scope.ts`: composing,
  recording, hidden windows and top overlays keep keys; editable, composer,
  terminal, browser, command-center and other are distinct focus scopes.
- `components/keyboard-shortcuts-dialog.tsx`,
  `keyboard/shortcut-help-search.ts`,
  `screens/settings/keyboard-shortcuts-section.tsx`: searchable grouped help;
  per-row bind, clear, reset and reset-all; chord capture/backspace/cancel.
- `command-center/registry.ts:1-119`, `results.ts:155-379`: owner tokens,
  duplicate refusal, registry ordering, visible-before-keyword matching,
  contribution sections before recent built-ins, five recent rows per category.
- `command-center/command-center.tsx`, `interaction.ts`: wrapped navigation,
  stable active identity, focus restoration, files scope and empty-query back.
- `command-center/root-registration.tsx`, `workspace-registration.tsx`,
  `agent-control-registration.tsx`, `plugins/command-center/registration.tsx`:
  root/workspace/composer/plugin sources use the same dispatcher as keybindings.

## Structure and data flow

- Pure scope policy and typed registration ownership live under the existing
  view shortcut/palette modules. No I/O or GPUI in those modules.
- A keyboard press resolves through the single shortcut registry and context,
  then a typed action reaches the owning feature's adapter. Pane actions use
  W12's `run_layout_action`; provider choices use the composer setters; file results
  use the file owner's API. Missing owners are reported, never silently faked.
- The command registry holds source-owned contributions and typed targets.
  Its projection reuses `view::matcher::Query`; a presentation is an action or
  a nonempty choice path. Source removal is generation guarded.
- Existing preferences remain the local override write path. No node RPC,
  protocol version, store migration or dependency edge is needed for keyboard
  mechanics or contribution projection.
- New UI vocabulary uses the generated Fluent catalog in EN and KO.

## Deliberate differences and pending rulings

The old app retains page-era shortcuts (N, G S, fixed Alt+digits), whereas the
package requires Paseo defaults. Coordinator authorization permits updating obsolete default assertions; the
mapping and new regression cases are recorded below. Unrelated cases remain. Existing
GOAT queue actions remain available as explicitly named extensions.
Paseo represents choice paths as flat query results; the survey's nested-page
requirement will use those same choice paths, with a back action and query
reset, rather than a second contribution registry.

## Validation and closeout

Per-commit narrow view/app tests and clippy through cargo-slot; final one native
remote gate. End-of-package fake-fleet captures at 1280x900 and 390x844, plus
800x600 for layout-sensitive changes, both themes and EN/KO. Never use live
preview ports or real agents. Review V4 receives one range, gate and captures;
rows are marked passed only after the corresponding behavior is verified.

## Agreed integration seams (preparation, not verified behavior)

- W1/A3: `PaletteMode::{Commands,Messages}`, `PaletteTarget::Messages`,
  `palette/messages.rs`; preserve mode invalidation and the extracted
  `palette/commands.rs`. Skeleton candidate `665d1ff2` is unreviewed.
- W12/C2: `view::layout::LayoutAction`; app adapter
  `Shell::run_layout_action(action,window,cx)->bool`. This supersedes the
  archived duplicate's pane proposal. One keyboard interceptor only.
- W12b/B2: `shortcuts::composer::ComposerAction`; proposed app adapter
  `Shell::run_composer_shortcut(action,window,cx)->bool`. No Send default:
  `sendShortcut` and current queue/steer behavior belong to the composer.
- W19b/E3: `shortcuts::desktop::DesktopAction`; app adapter
  `Shell::run_desktop_shortcut(action,window,cx)->bool`. Defaults include
  new window Mod+Shift+N; fullscreen Ctrl+Meta+F on macOS, F11 elsewhere;
  zoom Mod+=/Mod+-/Mod+0. Mod+W first addresses the current workspace tab,
  then falls back to closing the window only if no tab close was handled.
- W2/A4: `Shell::open_import(window,cx)` opens a modal over the current
  route; title from `shell.words.history().title()`. Close the palette first.
  Skeleton `367685a8` is not yet consumer-ready.
- W11/C1: project/workspace identity and pin/rename operations use
  `view::registry` and `link::registry`; the project action opens a scoped
  draft without implicitly registering or trusting a project. No ready hash.
- W13/C5: existing `Shell::code_find_files(key,query,limit)` in
  `areas/workspace/code/elsewhere.rs` reuses file.find; a callback independent
  of the composer is pending. Do not duplicate file I/O. Node suggestions are
  pending; requested-parent file.list is the approved temporary fallback.
- W16/F6: plugin targets retain `ContributionAddress` including exact node,
  plugin and runtime generation; OpenScreen and Invoke actions remain typed.
  The plugin owner rechecks online status and generation at dispatch.

The desktop action contract alone is committed at `8e9c8674` for W19b's early
consumer validation, explicitly unverified when handed over. This is not a
package completion or a passed tracker row.

## Preparation evidence and explicit bounds

Package base `8d5513e4` was merged without conflicts. The Messages seam was
merged from W1's `665d1ff2` candidate; it remains an unreviewed dependency.
The two merge conflicts retained both ARCHITECTURE appendices and every
palette module registration. Existing head cursor tests stay unchanged; typed
source ids use an explicit identity API sharing the same comparison/wrap rules.

The new sequence model accepts up to 16 steps / 256 bytes. Paseo has no such
capture bound; this explicit local preference/capture bound prevents unbounded
retained data, preserves all defaults and reports refusal without replacing the
previous capture. Native editing and quit/app-switch/browser-reserved keys keep
the app's existing refusal floor; native New/Tab/Close accelerators are bindable.
These restrictions are a stated difference from Paseo's unrestricted recorder.

First narrow preparation runs exposed a missing per-key translator comment,
then generic cursor inference in empty-list head cases and two lint issues.
The comments and cursor API were corrected without changing those head cases.
The interrupted second submission had no registered run; the build-tools owner
confirmed its exact SHA and authorized one pinned resubmission. Its result is
recorded under the package scratch logs, not treated as an app/package pass.
No keyboard/command-center tracker row is passed from these model checks.

## Canonical keyboard slice (2026-11-15)

The registry now contains the 76 reference binding variants in their original
order, with typed predicates and payloads. Native desktop additions and unbound
non-Mac pane entries are named separately. Persisted action aliases migrate in
memory; writes normalize aliases atomically so resetting a canonical row cannot
resurrect its historical alias. Unknown ids and other-platform settings survive.
The view production-library Clippy command passed in canonical-view-clippy-r2;
all-target checks, tests and app wiring remain pending at this checkpoint.

Default assertion migration: `/` and `N` are no longer product defaults; `G S`
is only an explicit sequence fixture; workspace digits use the primary modifier,
not fixed Alt; the inspector is primary+E, with Control+backquote as a hidden
alias. Jump actions are rebindable and captures normalize a final digit to Digit.
Question-mark input retains its logical identity. Legacy parser adapters remain
for existing consumers; the active interceptor delegates to the shared engine.

Settings uses canonical per-binding rows. Its recorder keeps multiple steps,
Backspace removes one, Escape is recordable, and the visible Cancel button or
navigation cancels recording. Confirmation lists every overlapping binding;
reassignment preserves unrelated alternatives and checks changed conflicts again.
The native host/editing refusal floor is explicit in sequence_capture.rs. The
popup key-context marker is applied by existing kit popup/dialog/picker controls,
so their focused controls retain keys without another listener or overlay guess.

S2's exact theme/input dependency eddce254 is now V1 PASS and remains in history.
The three GPUI guard regressions remain, with an explicit G S fixture for pending
sequence setup. The announced next shared base is 6c782cb6; merge is pending this
checkpoint. No tracker rows are passed and no whole package gate has run.

## Source registration and shared bases (08:4x checkpoint)

Shared bases 6c782cb6 and 3cd0d3fc are merged (e4c7d157 / f12d347b).
Token/size conflicts retained the actual lower counts; no allowance was raised.
The source registry now captures owner generations when presenting a result;
resolve(id,owner) refuses replacement/removal before activation. The native
adapter in palette/source.rs captures feature-owned typed values in an Rc
callback rather than interpreting plugin RPC or filesystem strings in the UI.
Feature sources use Shell::register_command_source/remove_command_source and
Contribution<Command>; callback, shortcut and nested-page targets share the one
existing palette renderer and keyboard cursor. Root static commands are projected
through that registry, followed by existing session recency. Messages entry now
comes from a contribution and retains W1's mode/dispatch/invalidation branch.
This checkpoint has not passed app compilation or the package gate.

C2 authorized the tiny terminal owner query in presentation.rs, committed alone
as 8d4b75b38. It checks actual focus + a retained opening + Session route; it
makes no inference from selection or phase. Its GPUI focus/route/closed fixture
is written but not run. The central interceptor now calls that owner query.
Browser focus is explicitly pending W14's actual renderer and resource revision.

The first immutable canonical view test run 9013121a completed with 892 passing,
16 failing. Fifteen legacy expectations and one new Alt+digit assumption needed
updates: Alt+digit now addresses a tab, primary+digit a workspace; Mod+Shift+T
now creates a terminal instead of being unbound; captures/index editing and the
bounded edit transaction follow the new registry. The old editor helpers remain
compatibility consumers, while Settings uses SequenceCapture/rebinding. The
compatibility listing's explicit platform now selects the correct platform's
canonical definitions, rather than displaying Linux keys in a Mac fixture.
Follow-up tests are pending. No unrelated behavior test was removed.

## Fluent migration and owner-facing projections (09:1x checkpoint)

The legacy shortcuts.rs action/group arrays are removed. Compatibility listing
helpers take ShortcutWords::new with an explicitly prepared Translator; generated
Fluent labels and full variable sentences replace the old indices and Rust
format strings. Native UI already reads shell.words directly. Sequence separators
are prepared with spaces at the formatting event, not concatenated during paint.
Native conflict persistence failures retain the recording and old bindings and
show a translated notice instead of silently discarding the attempt.

palette::builtins projects generic, owner-supplied identities/targets for the
Workspaces, Agents and Files categories. Workspaces and agents use the existing
Query scorer, visible-field priority, match-tier then recency, and a shared five
recent-row limit. Whole PR/MR-number queries use the existing forge PullNumber
wire type and outrank ordinary text hits. File results preserve the file owner's
ordering. This is preparation for W11/S2/W13 integration, not live search proof.
No session-derived WorkspaceId, alternate file I/O or alternate matcher is added.
Named Rank constants fix the shared tenths convention: workspace -10, root 0,
models 10, thinking 11, modes 12, plan 13, fast 14, plugins 50.

Native source Clippy a1e71f0bf reached the view crate and found one unused-result
warning, corrected. The next immutable run 14213f023 reached app and found four
app issues (two unused imports, a wrong Glyph variant and a missing scroll trait
import); these are corrected in the following slice. No app-green claim yet.
The local cold app check exceeded the narrow time budget and provides no pass.
S2/other feature handlers still await their consumer-ready exact heads. W12 has
confirmed NewTab opens a passive chooser and guarded close is handled, preventing
fallthrough to W19's lower-priority window close. G1 is preparing its own opaque
repository/generation/session target and current-menu availability adapter.
