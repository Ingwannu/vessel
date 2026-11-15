# F4 command center and keyboard parity

Status: implementation in progress on gated shared base c5263270 (merged8c69b8243;
prior d18bb794 merged612b26df6,
prior dd13ce378 merged5a54c0aca and18eedad4 merged7e9cc3d09),
with exact
canonical value prerequisite e46cd57b and portable W12 producer 9480d4d0
(adopted as ad8f4931b). S2's exact theme/input dependency eddce254 is reviewed V1 PASS.
Canonical defaults, dispatcher, Settings/help and source projection are written;
combined strict checks, feature-owner integration and package verification remain open.
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

Every compile, test and Clippy run uses vessel-remote-gate on build-vm; no new
cargo/rustc or cargo-slot invocation runs on dev-host. Run narrow checks per
meaningful immutable source, then one final frozen native remote gate.
End-of-package fake-fleet captures at 1280x900 and 390x844, plus
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
  zoom is withheld until the lead's production viewport-scale capability exists.
  Mod+W first addresses the current workspace tab,
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

## Registered Git source and current checks (09:4x checkpoint)

C5 source0df7abd9 is merged as b59a2ce01 for the shared file-search controller,
explicitly not its whole-package pass. The file owner is adding the exact active
scope getter; the scoped file opener and live G4 admission remain pending. No
legacy global file opener is exposed as a scoped command-center operation.
G1 source47e39d988 is merged as df4f434be for its compiled opaque GitCommandTarget
and menu adapters. palette/git_source.rs retains an equality fingerprint of that
target and owner menu rows. It replaces its source token only on change, avoiding
stale callbacks on every animation frame. Only a matching primary Git ActionRow
with no unavailability and an enabled MenuItem is promoted to Always; remaining
entries are query-only. Callback execution delegates run_git_command, including
its exact repository/incarnation/session/pull-identity checks. No Git policy,
RPC call or connection resolver is duplicated. These dependencies still await
producer final package/runtime evidence; they are not declared passed by F4.

W12's1720 adoption was held by its owner after shared-line compilation failed;
NONE of its pane/plugin/proto branch was adopted here. Wait for its self-contained
checked successor on announced base5, preserving the one layout owner.

Clippy30b272c6 found one similar-name lint in builtins (scope/score), corrected by
renaming the scope parameter. The local view suite progressed through the changed
shortcut/catalog cases but hit its time budget on the terminal stress fixture;
two observed failures were obsolete expectations: Mod+0 now means Actual size,
and the legacy browser-floor helper refuses primary+digit before conflict review.
The actual Settings recorder remains SequenceCapture and permits index rebinding.
The compatibility overlap helper now delegates to the same wildcard/prefix policy
and no longer labels workspace navigation as a fixed action. No complete local
suite pass is claimed. From coordinator09:2x to12:30 all NEW Cargo work runs
remotely; already waiting/running local jobs are left alone. Checks stay batched
per meaningful slice, and one whole final native gate/review remains outstanding.

## Workspace badge policy (written, not checked yet)

The one root modifier observer feeds view::shortcuts::badges, which derives a
single modifier from the effective workspace Digit template. Multi-step and
multi-modifier overrides have no reveal key. One cancellable task reveals after
150ms; Shift, blur/hidden, preference edits, capture and IME clear the hold. The
listener and badge query share compiled definitions and their When predicate.
The sidebar calls Shell::shortcut_badges_visible(window,cx); S2 alone supplies
logical workspace target order (full filtered open groups, pinned first, with
first9 badges, including rows beyond ShowMore paint caps). Child-agent sessions
are excluded. No parallel target cache, defaults table or key interceptor exists.
A GPUI modifier-event/timer/preference-reset regression is written, not run.

The old session-list implementation of numeric/relative navigation is removed
from the core fallback: it must not masquerade as workspace navigation or include
managed child-agent rows. S2's real workspace owner will register these three
actions. Badge eligibility also requires an enabled registered workspace handler;
a pending owner cannot advertise numbers whose keys would do nothing. This is
an explicit remaining integration task, not a completed navigation claim.

Batched suite caf26ebec type-checked all changed view/kit production and test
sources and reached App. It reported App lint issues (root-builder length, an
exhaustive target match, pointer cloning/semicolons, a small dispatcher argument,
unconsumed focus results and help scrolling arithmetic). The root source was
split into palette/root.rs, scrolling now uses GPUI item positions, and the other
issues were corrected. This is not an all-target green claim. The next immutable
suite includes the badge and actual workspace-owner admission changes.


## Pane owner adoption and compact boundary (10:2x checkpoint)

The held W12 producer 1720 was never adopted. Exact canonical proto values e46
are merged unchanged, followed only by the portable native producer 9480. This
preserves both C5 Intelligence and Layout answer registrations; neither owner
module is replaced. The producer's changed-source Clippy evidence is scoped to
its own head, not evidence that the combined command package compiles.

[Decision Log]
- Purpose: connect one keyboard dispatcher to the reviewed pane owner without
  recreating pane policy, resource creation or lifecycle management.
- Constraint: Mod+W can have a desktop fallback, while a refused/cancelled pane
  close must remain handled. The owner is the only source of that distinction.
- Alternatives: a second key listener or direct layout mutations would duplicate
  scope and close policy; both are rejected.
- Choice: a closed Action-to-LayoutAction translation and priority100 owner
  callback. The desktop fallback is lower priority; missing index payloads are
  refused. NewTab opens the owner's passive chooser and creates no agent.
- Impact: keyboard pane commands work only in the owner's Session scope. Palette
  workspace commands still await an owner captured-scope validation seam, so
  stale workspace actions cannot silently apply to a newly active workspace.

Paseo keyboard/availability.ts excludes iOS/Android (constants/platform.ts)
plus compact xs/sm below600 (constants/layout.ts). The native interceptor,
capture, help opener and shortcut chips share available(window); pointer-opened
command center remains usable compact. Tests cover390 refusal and600 admission,
and a real Mod+W refusal reaching the pane owner rather than window fallback.
These new tests are written but not executed at this checkpoint.


## Files UI lifetime preparation

C5's file_search module remains the one allocator/acquisition/cancellation path.
The new pure palette::files model owns only UI debounce (100ms, zero for empty
files scope), typed owner-supplied scope, retained same-scope rows and monotonic
UI tickets. Query edits invalidate old results before dispatch; A→B→A and
close→reopen cannot revive old callbacks. It is not a connection lease or a
workspace resolver. Native file scope getter/opener integration is still held
for W13's checked owner packet; no duplicate I/O or legacy opener is added.


The d8cb9f67 remote all-target suite compiled changed View/Kit/Link and reached
App, then failed one unchecked Duration subtraction in a badge regression and
one mutability mismatch: the kit's authoritative focused-composing query needs
mutable Window/App to inspect its text engine. The badge visibility helper now
accepts those references, preserving the actual IME query without a second
composition cache. No new kit or field API is introduced. This remains failed
suite evidence until the next source passes; no app-green claim follows.


Hidden-window admission is checked explicitly at the same interceptor before
Settings capture or matching. Window visibility is not inferred from viewport
width. Existing S2 composition/CharacterInput guards remain first and unchanged;
the new actual visibility-event regression checks hidden keys cannot cycle the
theme, and visible keys resume through the same handler. It remains unexecuted
until the next native test suite.


A source audit found Settings Shortcuts had an additional shared `phone` gate
(768) despite the keyboard boundary being600. The section keeps the shared
Settings renderer signature but ignores that presentation flag; actual viewport
availability alone chooses unavailable vs binding rows. The new real Settings
render regression uses700 (phone form but keyboard available) and599 (hidden
binding rows). Global PHONE and E2 navigation/form policy remain unchanged.


Settings builds one effective-binding snapshot per page render and passes it to
all rows. Long user sequences wrap between whole chords inside the existing row
width using kit::command::key_badges for every cap. No new badge style/control
is introduced; final visual inspection still owes long-sequence layout evidence.


## Shared connection admission merge

Announced shared line2e95 is merged without importing a separate node head.
It includes base5, exact e46 canonical values, the same portable W12 producer,
and G4's shared connection preflight. Conflict resolution preserves both C5
Intelligence and Layout variants/handlers/modules, retains both architecture
appendices and uses G4's extracted common peer-test helper. Every size allowance
stays equal or decreases. The old Messages fixture now explicitly carries no
connection_epoch; fake online reach/node data never invents an admitted lease.
The current combined Clippy run bc790536 predates this merge; it cannot prove
that this combined head compiles. No whole package gate has started.


Canonical NewWorkspace (retained Action::NewSession identity) no longer falls
back to opening a global agent Draft. It requires W3's actual workspace-create
owner handler, just as numeric workspace navigation requires S2/W11 projection.
Missing owners remain unhandled rather than creating a different resource.

Preference edits currently report local validation/application only. W19b owns
the sole shared desktop preferences Coordinator/Client/Snapshot write receipt,
rollback and per-window rebase path (source71e2c94ec, not checked at this point).
F4 does not introduce another writer or interpret SavePrefs queueing as disk
success. The checked common coordinator and rollback invalidation hook remain
a required dependency before claiming persistence-failure parity.


The bc790536 remote suite failed doc_markdown in the updated key-preference
comment, before reaching App checks; that comment is corrected, with the same
formatting applied to the interceptor's CharacterInput name. This is not an
App compile result. Its replacement will also verify the official2e95 union.
The two10:35 cancelled jobs are confirmed infrastructure interruption (VM
resize/restart), not source failures. Per T/coordinator ruling, the latest-source
Clippy and View tests each have one replacement; old143 logs remain retained.


## Shared actual-close focus restoration

Palette/help now reuse kit::focus::ReturnFocus, already introduced and reviewed
for shared overlays. Each open cycle captures a weak opener once; actual close
restores only while the overlay's search owns focus. A removed opener cannot be
kept alive by the modal; missing focus falls back to the existing composer entry.
A newer focused field is preserved. This replaces duplicated strong-handle
restoration, without changing kit internals or creating a focus registry. The
new actual GPUI regression covers removed openers and newer focus for both
surfaces. It is written, not yet executed.


The root projection now uses stable canonical new-workspace and settings ids,
with reference ranks1/6 in Actions. Settings subpages and old host/board/add-host
destinations remain query-only extensions; they no longer fill the empty list.
Their presence does not stand in for unready Home/History/Schedules owner routes.
Messages and Theme remain explicit existing extensions. A native projection
regression checks the empty defaults, Settings rank and searchable Appearance.


## Outstanding owner integrations (source audit, not completion)

| Surface | Owner boundary | Current evidence / remaining work |
| --- | --- | --- |
| Workspace creation | W3 new_workspace_current | Source1f95, checked packet pending; no global Draft fallback |
| Workspace rows, navigation, project create | W11 registry plus S2 visible projection | Canonical values present, no checked native activation packet |
| Numeric sidebar badges/order/grouping | S2 owner handler and projection | One effective modifier timer written; targets and both-sidebar adapter pending |
| Pane palette actions / focus-mode exit | W12 LayoutActionContext/run_at | Checked30a context/exit source adopted in sharedline; native owner source and dispatcher wired, union checks pending |
| Root Import | W2 open_import | Source0eb3 all3clippy0, owner tests/review pending; not adopted |
| Root History | W21 Route::Sessions | Screen owner packet pending; no Archive substitute |
| Root Schedules | G2 Route::Schedules | Source-only, physical admission repair pending; no Settings substitute |
| Files search/open | W13 FileScope + FileLocation | Pure UI state and existing C5 acquisition available; checked actual connection-scoped opener pending |
| Composer actions and choices | W12b owner setters + resident/W3 getters | Typed action contract exists; actual owner-generation admission and adapter pending |
| Plugin items | W16 exact contribution registry + transport | Canonical values and narrow registry source exist; executable product adapter absent |
| Browser focus/forwarding | W14 exact resident browser revision | Owner renderer/admission callback pending, no guessed browser context |
| Desktop shortcuts and persistence receipts | W19b shared dispatcher and preferences coordinator | Source be31d4d65, checks pending; no second writer or duplicate failure toast |
| Messages mode | W1 existing Searching controller | Initial source665d preserved; window-lifetime successor not checked/adopted yet |

All rows remain skeleton. View/Kit/App source and behavioral tests must pass on
the combined actual head before the final frozen gate/captures/V4 submission.


The command center's existing frame/search receives bare Up/Down/Enter/Escape
and empty-page Backspace before global rebinding, matching the reference top
overlay priority. The interceptor yields those events and clears prefixes; it
does not create another controller. A user override of Palette to Down cannot
close the list instead of navigating. Escape always closes the center, including
nested pages; Back and blank-query Backspace handle parent navigation. A real
key-event regression covers that override and nested Escape closure.


Routing audit against keyboard/route-shortcut.ts distinguishes sidebar scope
from the sidebar.* action prefix: PinWorkspace, ArchiveWorkspace and right-panel
toggle route to Sidebar, while only the three sidebar toggles exit focus mode
AFTER a handled result. A typed exits_focus_mode helper expresses that prefix
policy without string matching. The actual idempotent exit call waits for W12's
checked follow-up; ordinary workspace creation/pin/archive must not use that hook.
Canonical NewWorkspace defaults are Cmd/Ctrl+N; OpenProject is Cmd/Ctrl+O.


## Checked W12 context adoption and layout contribution source

Merged the WHOLE exact follow-up9e18..30a4396f, preserving exact canonical e46
and unrelated owner source. W12's evidence is bounded source adoption plus exact
View/App all-target Clippy and38 App/View tests; it does not prove F4 integration.
The new workspace-layout source captures LayoutActionContext for each callback,
rechecks via run_layout_action_at, and preserves stale handled-close behavior.
It projects query-only tab/pane actions with reference ranks and current labels,
using the same Action-to-LayoutAction mapping as the sole keyboard dispatcher.
New tab is explicitly a chooser extension, never a new-agent placeholder.
Handled sidebar toggle actions now call the owner's idempotent layout_exit_focus.
W19's literal cfg(test) preference-cache staging stays unchanged and disabled.

R8 reached App and found three phone-parameter references plus one private-field
test access. The shared Settings phone flag still controls menu/dialog geometry;
it is ignored ONLY for keyboard availability. Those references are restored,
and the test uses the public availability boundary rather than a private flag.


Shortcuts controls also use desktop menu/dialog presentation whenever the
section is available. The shared Settings phone-form argument is deliberately
unused: at700 the source is noncompact despite global PHONE768. Availability
has already excluded native mobile and below600, so the existing kit controls
receive noncompact presentation; no shared breakpoint or renderer is changed.


The UI file-query model now distinguishes FileQuery::All from Matching(FindQuery).
The existing protocol explicitly rejects blank FindQuery, so passing an empty
wire query was invalid and its prepared test would fail. Empty-scope acquisition
must come from W13's scoped owner listing path (requested-parent file.list is the
approved temporary difference), not local enumeration or a weakened protocol.
The model's immediate empty listing case remains the same behavior with an
explicit valid state. W13 is notified; native All acquisition remains pending.

Package base6 18eed is merged from the official app line, including the exact
W12 follow-up. No standalone node head was merged. The lead warns its native
lock is stale and supplies a correction next; F4 invokes no --locked operation
and does not generate a replacement lock locally. Current queued checks are
older immutable source evidence and do not prove this latest union compiles.


## Code editor composition boundary

W13 approved the one registration line in CodeEditor::new. The existing weak
TextField composition registry is extracted into kit/field/composition.rs and
extended with EditorState; the public focused_composing API and first interceptor
guard stay unchanged. One registry now covers form fields, composers and code
editors, prunes dead handles and preserves registrations made during a query.
No route/displayed-file guess, duplicate cache or composer_hooks change is added.
The actual CodeEditor constructor + marked-text regression checks focus switches
and commit. It is written, not executed yet. Field callback factories and K's
pending callback/state split must be preserved when integrating their deltas.


Exact cc7883ca full View library run completed:1038 passed/1failed/0ignored,
with the sole failure in the new file UI empty-query fixture (invalid FindQuery),
already corrected by explicit FileQuery::All at c198fb75. All shortcut/default,
compatibility migration and other View tests passed in this immutable run.
This is NOT a full green View result and excludes later source; current fixed
model and App/Kit GPUI changes still require execution. Composition extraction
lowers field.rs allowance635→587; no file budget is raised.


Git contribution fingerprints and activation now use the shared MenuItem
selectable() query, not only disabled: Pending/Success rows can be nonselectable
without setting disabled. Activation rereads that same owner-provided item and
then delegates run_git_command; no availability policy or RPC is copied. This
is UI admission only, not physical connection before-poll/post-await fencing;
G1's checked isolated connection-admission correction remains pending.


L reviewed exact8b4bd738: WORD/KEY/CONTEXT ACCEPT for all4 catalog/helper blobs
and93 EN/KO keys, including57 action labels. Packet:
/tmp/work/m12-i18n/ko-review-w22-shortcuts-8b4bd738-20261115/result.json.
This is source wording evidence only. Applied L's nonblocking comment-only
clarifications for physical chord-step vs alternative separators and exact legacy
sentence variables; Fluent values, keys and interpolation contracts are byte
unchanged apart from comments. No second wording gate was requested. Dispatch,
transaction/capture retention, App/Kit execution and package acceptance remain
separate. Existing whole bidi goldens retain FSI/PDI.


Actual overlay admission now precedes Settings capture as well as matching.
The same named rendered kit context pauses the recorder without losing its
accepted steps; after the menu actually closes, capture resumes. A real kit-menu
focus/context GPUI regression uses the production interceptor, verifies no key
is recorded behind the menu, then verifies recording resumes. Existing S2
CharacterInput/composition guards remain first and unchanged. S2's hierarchy
owner reuses the exact same marker and adds no Menu scope/cache/listener.


## Viewport capability and unsupported zoom ruling

Adopted ONLY W12's exact4-file viewport leaf f549b746 as57929abdb, not the
staged resident/resources branch. V1 permits this bounded adoption based on
exact-code strict3-crate Clippy and viewport1/0 evidence in producer1d93; no
standalone f549 run is claimed. Palette pane availability now consumes owner
layout::mode(window)==SplitPanes; no copied600/768 policy remains. The source's
single-pane focus-mode command uses rank26, split-pane focus-mode rank62.

Coordinator/W19b rule: native zoom is unsupported until a real production
per-window viewport-scale seam exists. Removed ONLY four speculative native
extension registrations (ZoomIn alternatives, ZoomOut, ActualSize); canonical
76 reference bindings remain. Typed intentions/labels remain for later support,
but no keys, help/Settings rows, chips or conflicts advertise zoom now. Future
native dispatcher admission must use the desktop owner's supported() capability.
No test-only scale or rem-size substitute is used. The prior Mod0 extension
assertion is updated and a platform projection regression covers this capability.


Exact6d336854 combined all-target Clippy reached App production/test analysis
and failed only5 strict lint findings: row function102lines plus test import,
qualification and two semicolons. Extracted its existing menu button rendering
into one local function; no second control/style/callback is introduced. Other
findings corrected without relaxing lints. The next immutable combined check
also includes the later composition registry, capability policy and shared lock.

Remote App binary tests on exact5a54c0aca executed108pass/2fail/2ignore;
the two failures exposed incomplete interaction fixtures. Pointer-opened kit
menus intentionally start without a highlighted row, so the recorder test now
navigates Down before Enter. The modifier test now creates the actual composer
focus path required by GPUI key dispatch and polls its spawned timer before
advancing virtual time. These fixes preserve production menu, focus and150ms
policies. Kit tests were not reached after the App failure; no kit pass is claimed.

Badge visibility now queries the same actual Window context stack and exact kit
VesselOverlay identifier used by the interceptor. A focused popup suppresses
both badge queries and new modifier holds; no inferred menu flag or focus cache
is introduced. The real modifier regression additionally opens the production
Settings row menu and checks its rendered marker plus hidden badges. This closes
the separate modifier-only path, which cannot rely on a later key-down guard.

Withheld the old ToggleSidebars-to-left-only core fallback. S2's source requires
one owner decision across both actual sidebar states; a left-only substitute
violates that behavior in an active workspace. ToggleSidebar remains the existing
left action. ToggleSidebars keeps its canonical binding but has no active native
handler until S2's checked sidebar_toggle_both packet is adopted and registered.
No sidebar state or lifecycle policy is copied into the dispatcher.

Native conversion audit: GPUI's platform/keystroke.rs18-32 carries typed key_char
separately from its platform-normalized base key (including Option and non-ASCII
layout fallback); our press_of keeps typed text first and derives fallback only
from that supplied base key. Linux platform.rs1418-1540 contains the upstream
XKB/ASCII normalization. No scan code is invented at the Shell boundary. An
actual GPUI synthetic event regression supplies a non-Latin typed character plus
base T through the production interceptor. This proves adapter behavior only;
macOS Option, Windows AltGr and actual OS layouts remain unverified.

Strict4-crate run wp-commands-narrow-20261115T120545Z-4dc236da ended rc1:
View/Kit/Link and App analysis were reached, but the new App overlay fixture
omitted the px import (two E0425 occurrences). Added the missing test-only import;
no product behavior or lint setting changed. The already queued same-source
App/Kit test job123242-db95665d was verified by its ticket and local submit PID,
then canceled queued-only to avoid spending a slot on the known compile failure.
Replacement waits for that exact job's terminal state; all original logs remain.

Reference workspace-file-search.ts119 uses JavaScript trim to choose immediate
listing vs100ms debounce, and command-center.tsx364 uses it for empty-scope Back.
Files UI parsing and nested-page Backspace now reuse matcher::Query::is_empty
instead of Rust trim or byte emptiness. This shares the existing ECMAScript
whitespace predicate without changing matcher visibility/body/scoring or adding
a helper. BOM is blank; NEL remains nonblank and the stricter existing FindQuery
grammar refuses that control character instead of inventing All. Pure Files and
actual GPUI nested-page fixtures cover both. Parsing stays UI-owned when W13's
canonical FileQuery values leaf replaces the temporary local enum.

W13 pure-values receipt received: exact59763344f99fd0a404a367714bd72171fa78f778,
run wp-files-query-values-20261115T120324Z-59763344, native-fonts0 and actual
View/Proto test1/0/0 (949filtered). Adopted ONLY048b62108+doclint36260cc66 as
9bbc4e0c4+b35e22ea8. Resolved the one module-export conflict by retaining every
existing explorer/file-ticket export plus the new file_query line. The canonical
file blob remains byte-identical to the producer. Removed F4's duplicate enum;
palette::files re-exports the owner value and keeps parse_query UI-local. No
FileScope, transport, root query or opener blobs came with this pure handoff.
Combined current-head checks remain owed; no standalone Clippy/whole/runtime C5
approval is inferred from that single producer test.

Adopted S2's exact isolated menu producer dd13ce378..2599d051c by merge95555d7aa,
preserving its original commits. V1 bounded source/Kit narrow pass covers private
opening leases, stale recognized-key/AboveDialog containment and pointer default
focus fencing (Kit173/0/0 +Clippy0). The only Kit delta against the producer's
five source/test files is F4's existing one-line VesselOverlay marker on the
actual focused root; both that marker and S2's new capture focus fence survive.
README retains both independent additions. This does not import S2's unready
Main F4 branch or activate ToggleSidebars. Current F4 union checks remain owed.

Lead confirmed d18bb794 is the published package base, locked native gate
spike-20261115T125314Z-d18bb794:1415/0/2. Merged the exact shared line as612b26df6:
W4 FieldId/source callbacks and the existing composition extraction auto-merged
through the same central line/multiline factories. All engines still register
once; no input callback or identity implementation was copied. Shared README
conflict retains both complete documentation additions. Field's combined product
size is567, ratcheted down from587; picker remains828, never raised to the incoming
older829 record. W12 effective target getter now arrives from the official line;
it grants no authority or global Draft fallback. This base's gate is prerequisite
evidence only; changed F4 union checks still remain open.

Lead subsequently announced c5263270 native1431/0/2; adopted exact shared line
as8c69b8243. S2 menu commits already present merge cleanly without replay. The
additional delivery guard/export, follow and queue modules retain their literal
cfg(test) staging attributes. F4 activates none of them and gains no live
resident/follow/connection delivery authority from this merge. The announced
next node-only base8 remains under gate and is not consumed yet.

Exact a67511993a6a85757bba60ba12934cb8649bc18d combined strict Clippy is GREEN:
wp-commands-narrow-20261115T124801Z-a6751199, all targets of View/Kit/Link/App,
-D warnings, native-fonts0/native-cargo0. The same-source actual App/Kit test
run124814 remains in progress. This source predates the later whitespace,
canonical FileQuery, S2 menu and official d18/c526 union, so none of those later
changes inherit this green result. A control-only patch/byte manifest for the
existing composition extraction is recorded under the package evidence folder;
consumers retain W4 FieldId and W13/K callback factories instead of copying the
older whole field file. No standalone control gate or V4 acceptance is claimed.
