# F4 command center and keyboard parity

Status: preparation on the announced 1d1f11df app head; package base and S2's
reviewed theme/input correction are pending. No node-backed action source is
implemented against an unannounced base.

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
  W15's `pane_command`; provider choices use the composer setters; file results
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
package requires Paseo defaults. Coordinator ruling requested before updating
obsolete head-test assertions; unrelated tests remain untouched. Existing
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
