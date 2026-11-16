# F4 command center and keyboard parity

Status: implementation in progress on announced base10 3e7820e3. The latest
checked F4 slice is exact547b2d72e (App strict0 and App139/0/2); the newer
base10 union still requires consumer checks. Canonical defaults, dispatcher,
Settings/help and source projection are written. Feature-owner integration and
final package verification remain open; no command-center or keyboard tracker
row is passed. No new node RPC is needed for the mechanics; resource and RPC
admission stays with feature owners. Historical source/evidence is recorded below.

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

Every compile, test and Clippy run uses vessel-remote-gate on the build VM; no new
cargo/rustc or cargo-slot invocation runs on the development host. Run narrow checks per
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
kept alive by the modal. A missing/dead opener stays blurred; a newer focused
field is preserved. This replaces duplicated strong-handle restoration without
changing kit internals or creating a focus registry. The earlier a675 checkpoint
used a route-level composer fallback; B2's later policy explicitly withdrew it.
The updated actual-close regression is queued. Weak liveness does not establish
visibility of a retained-but-hidden opener; that case still needs domain admission.


The root projection reserves canonical new-workspace rank1 for A1 and uses
canonical settings rank6 in Actions; the unready legacy creation row is omitted. Settings subpages and old host/board/add-host
destinations remain query-only extensions; they no longer fill the empty list.
Their presence does not stand in for unready Home/History/Schedules owner routes.
Messages and Theme remain explicit existing extensions. A native projection
regression checks the empty defaults, Settings rank and searchable Appearance.


## Outstanding owner integrations (source audit, not completion)

| Surface | Owner boundary | Current evidence / remaining work |
| --- | --- | --- |
| Workspace creation | W3 new_workspace_current | Source1f95, checked packet pending; no global Draft fallback |
| Workspace rows, navigation, project create | W11 registry plus S2 visible projection | Canonical WorkspaceId and announced registry values present; a checked native owner projection/activation packet is still absent |
| Numeric sidebar badges/order/grouping | S2 owner handler and projection | One effective modifier timer written; canonical target projection remains pending. The corrected both-sidebar owner is adopted with the real caller and checked focus-exit regressions |
| Pane palette actions / focus-mode exit | W12 LayoutActionContext/run_at | Checked context/exit source adopted in shared line; actual sidebar focus-exit union cases passed in R22. Resource effects still require their owner admission |
| Root Import | W2 open_import | Announced owner opener adopted; rank2 import-session contribution and receiving Import regressions pass at f49784f8. Native first-use capture remains owed |
| Root History | W21 Route::Sessions | Screen owner packet pending; no Archive substitute |
| Root Schedules | G2 Route::Schedules | Source-only, physical admission repair pending; no Settings substitute |
| Files search/open | W13 FileScope + FileLocation | Pure UI state and existing C5 acquisition available; checked actual connection-scoped opener pending |
| Composer actions and choices | W12b admitted setters + canonical view::options classification | Controls consumes actual owner reports only; choose constructs without validating fresh membership; real target/report/pending/editability validation and adapter pending |
| Plugin items | W16 exact contribution registry + transport | Canonical values and narrow registry source exist; executable product adapter absent |
| Browser focus/forwarding | W14 exact resident browser revision | Owner renderer/admission callback pending, no guessed browser context |
| Terminal composition admission | C2 actual terminal IME/preedit owner | No production EntityInputHandler/marked-text query yet; keep existing InputPreference + shared Kit guard, no composing=false proof |
| Desktop shortcuts and persistence receipts | W19b shared dispatcher and preferences coordinator | Shared keyboard producer1d908fa8 has V4 row295 bounded source PASS and waits lead announced receiving green. E3 adapter/receipt consumer remains unadopted; no second writer or duplicate failure toast |
| Git/PR command effects | G1 captured owner target, row policy and physical admission | F4 registration is staged under cfg(test) only; existing47e toolbar dependency remains separately unpassed. Original-connection/root/ticket write-owner packet is pending |
| Messages mode | W1 existing Searching controller + W12 session-open admission | New root entry, mode selector and transition are cfg(test)-staged; one window controller/rendering bodies preserved. Original-context opening owner remains pending |

These rows distinguish adopted local behavior from unavailable owner effects.
The full combined package still owes its final frozen gate, inspected captures
and V4 package review; individual source handoffs do not close F4.


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
the Korean-review packet `ko-review-w22-shortcuts-8b4bd738-20261115`,
`result.json`.
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

Same exact a675 App/Kit run wp-commands-20261115T124814Z-a6751199 is now GREEN:
App112/0/2ignored, Kit168/0/0; total280pass/0fail/2ignore. Actual production
interceptor/menu/capture, modifier150ms/preference reset, platform base-key and
CodeEditor marked-Hangul/focus-switch/commit regressions all executed and passed.
This closes the earlier two fixture failures and the control packet's previously
unexecuted IME case. Later c526/FieldId/menu/FileQuery/whitespace union remains a
different source and receives its own combined narrow validation next.

S2 hint integration boundary: primary_keys/binding_id are not independently
portable onto the old Action-level registry. Their per-binding Definition,
ShortcutContext and KeySequence identities arrive with the canonical action enum
and native exhaustive dispatcher. S2 keeps its existing one bindings_of adapter
until that coherent producer lands, then replaces that one adapter; chord_caps
already exists on the shared line. No second default table, fabricated binding
id, partial enum migration or whole moving F4 branch is handed over as a tiny
hint producer. NewWorkspace/NewSession and command-center/Palette typed variant
identities stay stable across the eventual adapter change.

Follow-up source audit found the same trim difference in help search and exact
PR/MR promotion: shortcut-help-search.ts68 and workspace-search.ts11-21 use
ECMAScript trim/whitespace. Adopted ONLY B2's agreed0e78261de matcher change
fn whitespace→pub(crate) fn whitespace; body, default Query, scoring and marks
are byte-for-byte unchanged. Both consumers now trim with that one predicate.
Existing behavior tables add BOM around/inside valid searches and NEL refusal
cases. No copied whitespace helper or borrowed B2 test pass is introduced;
these later changes require the next immutable narrow check.

Tracker674(default table),675(help),689(native editing/IME) now built with explicit a675 scoped-check and remaining union/gate/capture/V4 qualifiers. Both repository and live working trackers agree. Other owned rows retain skeleton; none is passed. These three implemented mechanics do not imply every feature owner/action or actual OS layout has been verified.

Combined c526/FieldId/IME/menu/canonical-FileQuery/blank-scope union exact
a52d5f106dddeaf10b81c9ab082ecd6a2500fa38 strict4-crate all-target Clippy is GREEN:
wp-commands-narrow-20261115T132327Z-a52d5f10, native-fonts0/native-cargo0.
Its App/Kit/View all-target tests remain in progress. The only later product
changes are the shared whitespace visibility and help/PR-query consumers;
those receive a View-only strict/test follow-up rather than repeating unrelated
App/Kit source checks. No final gate, captures or package review is inferred.

App/Kit/View run132327-a52 stopped after App112pass/1fail/2ignore: the new
nested Backspace regression exposed a real event-order bug. GPUI window.rs6198
dispatches bound Input actions before raw key listeners, so the search parent's
capture_key_down never gets TextField Backspace. Removed that ineffective raw
listener. After its existing IME/CharacterInput/overlay/recorder guards, the ONE
interceptor now invokes the existing palette_back only for the palette's admitted
bare empty-scope Backspace, then consumes it. Other local keys still yield to
the existing command-frame action handlers. The ownership query stays read-only;
no new listener/keymap/controller or kit control/API is added. The actual GPUI
fixture installs that production listener and covers empty/BOM/NEL/nonblank
queries. Kit and View tests were not reached after the App failure; no later
test pass is claimed from that run.

Commands breadcrumb/Backspace projection additionally requires Commands mode.
W1's mode switch may retain its other window fields; that must never expose or
navigate a Commands page while Messages owns input. A real keyboard regression
switches to Messages and checks that Backspace preserves its mode/page state.
Only F4 navigation projection/tests changed; W1's messages.rs/controller and
query-retention policy remain untouched.

Local Back remains available inside a pointer-opened compact palette even though
global shortcuts/help/capture remain unavailable below600. Availability clears
capture/prefix state, then the actual overlay check precedes only that existing
local Back operation; global matching still stands down. The same regression
runs at800 and390. Extracted the unchanged matching phase into a private method
to keep admission/local-control ordering readable and within function budgets;
both methods share the one engine/cache/dispatcher and listener.

View-only strict all-target Clippy exact83d3c5033d45fa465e6e7db00d37245488f84510
is GREEN, wp-commands-narrow-20261115T140914Z-83d3c503 (native-fonts0/cargo0).
That View source is byte-identical to the current View tree and covers the
shared whitespace visibility/help/PR-query changes. Later App Backspace/order
changes still require their own strict result plus the queued6d739b111
App/Kit/View regression run; no test or App Clippy pass is inferred from this.

B2 owner policy2942b7ca explicitly withdraws canonical FocusComposer's inherited
RouteSession/RouteDraft fallback until its admitted-target adapter exists. Removed
that core registration/effect and the now-unused global route helper. Palette and
help dead-opener close also no longer call it as generic focus permission: the
shared ReturnFocus restores its captured weak opener only while the modal owns
focus; a missing/dead opener stays blurred, a newer focus is preserved. Existing
live-opener restoration is unchanged. Updated the actual close regression and
added real Cmd/Ctrl+L missing-owner cases on both legacy routes. No W12 target
metadata, alternate getter, global New fallback or scope/IME exception is used.
The old a675 dead-opener test proved its earlier fallback behavior, not this
corrected owner policy; latest source checks are still owed.

Exact6d739b11197c692792afa77f081a47303a8c0662 regression run
wp-commands-20261115T142317Z-6d739b11 is GREEN:1480pass/0fail/2ignore.
App113/0/2, Kit182/0/0, View1045/0/0 plus View integration/example suites.
The corrected empty/BOM/NEL/nonblank Backspace at800/390 and Messages boundary,
current shared FieldId+CodeEditor IME, canonical FileQuery and default matcher
contract cases executed and passed. This source predates B2's FocusComposer
withdrawal; it does not validate the newer dead-opener policy.

Superseded queued App-only strict run144639-d27 was verified by exact ticket
and submit PID141432, canceled queued-only, and observed terminal cancelled143
before replacement. It could not check the newly agreed View/App focus policy.
Original logs remain. Latest View/App strict and App-only regression checks now
cover those changes; unchanged successful Kit/View suites are not repeated.

B2 clarified the remaining restore bound: weak opener liveness is not proof that
a retained field is currently rendered/visible. Dead/missing opener and newer
focus cases are covered by our tests; retained-but-hidden opener restoration
remains explicitly unverified until the domain's admitted restore guard is wired.
No blanket kit change, second visibility getter or metadata-as-permission is
introduced. Existing ReturnFocus is not claimed as B2 Visible/Focused admission.

W19 may add ONLY desktop_recording_started(window,cx) to its isolated older
Record path immediately before capture start/blur. The helper must retire the
existing desktop MenuState/cycle before recording so stale rows cannot restore
focus ahead of the command-time guard. Current F4 tree has no E3 producer and is
not edited to call an absent method. A concrete current-recorder/three-supported-
actions refusal regression is prepared in package evidence; native admission
assertion awaits the owner's normal production predicate, not a test-only hook.
The test is unexecuted and does not imply E3 capture/persistence readiness.

Exact02e4567ac37086dc438326d7693b6a20eab4d349 App binary regression run
wp-commands-20261115T145941Z-02e4567a is GREEN:114pass/0fail/2ignore.
The actual missing/dead-opener and canonical FocusComposer missing-owner cases
executed under the corrected B2 policy. Same-source View/App strict run
wp-commands-narrow-20261115T145941Z-02e4567a failed only doc_markdown on
TextField in the Backspace-order comment; corrected the markup without changing
behavior. Current union strict checking remains owed; no whole/UI/OS claim.

Announced package base8b c34299520 is merged after the prior narrow runs
finished. Merge decisions preserve one window PaletteArea.messages_search,
its incoming resident-replacement regression, existing immediate ticket invalidation
before debounce, and prepared selection before close. The obsolete environment
example renderer stays removed because Messages already has its real renderer.
The shared multiline constructor extraction/FormMinimum/FieldId is retained;
its ONE factory registers with the existing weak composition registry. No second
registry/subscription or constructor family is retained. field.rs falls below500
and its size exception is removed; picker remains capped828. Test-only resident
and delivery staging remains test-only. New union strict/tests are required;
previous source checks do not establish this merge.

Receipt integration inventory (pending W19 checked producer): edit/clear/reset
share shortcut_edits; ResetAll currently calls prefer directly and must join the
same owner receipt wrapper. The wrapper replaces that one call, never adds a
second SavePrefs. UI completion must match the current local edit/visit, pass
canonical snapshots through the owner acknowledge path, and distinguish Saved
from launch-only/refused/failed. Canonical rebase invalidates shortcut caches
without discarding a newer recorder. Existing actor/queue failure notices remain
central; local validation wording is not a disk acknowledgement. No speculative
receipt writer/state machine was introduced before the checked owner handoff.

Exactad852 union runs R16/R8 both stopped at App compilation before tests.
Two old file-tree SavePrefs callers needed the shared Box<Prefs> argument;
adopted ONLY that mechanical adaptation, matching W13 fb0f05295's two hunks,
without its unrelated staged close routing. Automatic merge also retained two
identical language_tests::with_commands definitions; kept one unchanged helper.
These are integration errors, not passing tests or infrastructure failures.
The shared field/multiline/composition code is unchanged by this correction.

W1/W12 clarified Messages navigation authority after base8b: prepared Destination
retains query/row selection but its alias-only Open/Command::Open cannot fence
physical successor resolution at the actor queue. The intended SessionOpening
resolve with original ConnectionContext, admit_session_open, real claim/context
and FollowBinding before hydration is not a checked live API yet. No weak UI
guard, current alias lookup or new open manager substitutes for that carrier.
Current Messages lifetime tests are source/UI evidence only; full navigation
acceptance stays blocked on the owner union. Commands remain independent.

Exactf0ebd607b96e65226113aadf7e53759491e36d59 is checked: App all-target
strict Clippy R17 rc0; R9 actual App/Kit/View lib+bins1360pass/0fail/2ignore
(App125/0/2, Kit186/0/0, View1049/0/0). The window-search replacement,
blank/BOM/NEL Backspace, no-route composer/dead-opener and current identified
multiline/CodeEditor composition cases executed and passed. Scope excludes
View integration/example suites, whole gate, captures, OS IME and physical
owner admission. The narrow control packet for base8b has unchanged tested
owned blobs; it is not a standalone leaf or package approval.

Announced base8d6ebdf8e8 is merged after R17/R9 completed. The shared picker
lease admission/key-handler signature is retained alongside the existing ONE
VesselOverlay marker and popup layer. README retains both independent notes;
picker size allowance decreases to804. Incoming W19 popover/W12 close-target,
attention/save-prefs extraction do not grant missing desktop or session-open
controller authority. New union checks are owed; f0eb results above stay scoped
to the preceding base8b combination. Gating e670 zoom source is not adopted.

Exact73881ef48dff51dd66bea047044db2e54b0ec433 base8d union is checked:
Kit+App all-target strict R18 rc0, run
wp-commands-narrow-20261115T161437Z-73881ef4; actual App+Kit lib/bins R10
331pass/0fail/2ignore (App126/0/2, Kit205/0/0), run
wp-commands-20261115T161437Z-73881ef4. Updated shared picker/dialog/popover
admission tests and actual window-search/Backspace/focus/CodeEditor IME cases
executed and passed. View1049/0/0 remains exactf0eb prior-source evidence, not
a repeated current-View execution claim. This is still a narrow union result;
no final whole gate, captures, V4 or missing feature-owner acceptance.

W19 native OS edit-menu wrappers088a6c266 are a source-only follow-up beyond
ec90's actual unit21 result. Retained popup/recorder UI tests do not establish
OS native Undo/Redo/Cut/Copy/Paste/SelectAll admission; eventual recorder union
must use the checked owner routing delta. No E3 moving branch is adopted.

Base8e82ca07a7 is merged without conflicts as3adb96247. New panel-retirement
hooks remain literally test-only, including registrations/notifications; they
do not authorize composer effects, backend operations or original-context open.
The metadata-only layout transaction now commits a clone only on success.
No resource clone/swap or independent palette lifecycle manager was added.
The preceding8d green checks do not validate this newer union.

G3/B2 confirmed the provider choice source contract: existing
view::options::{Controls,controls_with_effort,SelectControl,ToggleControl}
classifies ONLY actual owner-supplied SessionOptions. SelectControl::choose
constructs OptionChoice without checking current membership. B2 must refresh
the EXACT admitted target, actual current report value, pending/editability,
then invoke its existing owned setter. HarnessKind comes from the owned
HarnessRef/report. Settings offer caches, startup preferences and modern-provider
name/id joins are neither session report nor admission. Plan/Fast are reserved
group ranks, not static invented choices. B2 target/options effect adapter still
waits on the sole W12 resource cutover/live effect lease. F4 retains its one
source registry and ChoiceLocation controller; no second selector is added.

[Decision Log: do not project an unowned creation action]
- Purpose: an empty command center must not offer a first row that closes into
  an unavailable no-op while canonical workspace creation is unready.
- Existing assets/constraint: legacy palette_entries still supplies per-machine
  NewSession rows; W3 explicitly has no checked admitted canonical opener.
- Alternatives: dispatch the old global Draft route; keep the clickable no-op;
  or wait to publish the real source-owned contribution.
- Choice/why: omit only legacy NewSession from native-root; A1 will publish its
  admitted NewWorkspace callback through the existing registry, using the
  reserved canonical label/id/rank1. Mod+N and the76 default variants are unchanged.
- Effect: Settings is the first actual default root action. Removed the now
  unconsumed shortcut-command wrapper path; typed Command::callback and its
  owner revalidation contract are unchanged. A real Enter regression checks
  Settings opens and the unsent draft remains intact.
Superseded queued R11 exact062a was verified by its exact ticket/local PID2560684,
canceled queued-only and observed terminal cancelled143 before replacement.
Its old source would not test this projection correction. No failed build or
passing test is inferred from that cancellation; no extra whole gate begins.

C2 confirmed no checked production terminal IME/preedit owner, composing query
or registration seam exists yet. Its current pure frame/CLI byte serializer
does not provide EntityInputHandler/marked text. Existing terminal_focused is
actual route/shown/focus evidence only, not composition permission. Keep the
original CharacterInput guard and ONE Kit/CodeEditor registry; do not derive
terminal composition from phase/focus/session or add a false stub as proof.
A real future terminal owner must expose its scoped composition seam before
terminal-specific IME acceptance. No parser/clipboard/terminal edits or build
were added for this owner status; current App source check remains unchanged.

Creation audit also found the retained Legacy(NewSession) activation arm still
had an old global Draft/choose_machine effect, despite its row being omitted.
It now refuses through the existing unavailable notice. A retained-target
regression checks route, draft machine and unsent text all remain unchanged.
Thus neither current projection nor an old typed legacy callback can bypass
A1's admitted opener. No new handler, creation route or machine resolver is added.

Exact547b2d72e56deefd27ff62d59e257a4b65f1ab95 App strict R20 rc0 and App
BIN R13 actual139pass/0fail/2ignore. The new first-root Enter and retained
legacy creation target tests both executed and passed. No current canonical
creation owner, whole gate, capture or V4 approval is inferred.

W2 now provides checked bounded Import e1b73c9e..b27021b4 with actual031f
App/View/Link strict0 and import App2pass, V1 source clear; lead adoption is
still required before consumer activation. Primary reference root inventory
uses import-session rank2 (Home is rank3); an earlier coordination message
incorrectly said rank3, corrected before any implementation. Existing owner
modal/title/callback is reused; popup/automatic open stay parked.

Announced base9b a7aab9d6 is merged only after R20/R13 completed. Architecture
notes retain both the command-owner explanation and incoming registry/lifetime
contracts. SearchField keeps its existing TextField factory and thus the SAME
line composition registration; FieldId/FormMinimum and ONE multiline/editor
registration remain. Dialog header actions retain the actual overlay marker.
Production Window zoom availability alone does not enable F4's three extensions
before W19's checked adapter. New node registry values are canonical data, not
a checked native workspace navigation producer. New union checks remain owed.

Receiving9b R21 exact05f77 all4 native-crate strict is GREEN0. R14 actual
App stopped144pass/1fail/2ignore: incoming PreparedRefusal golden still
expected64 actions while F4's existing MAX_ACTIONS is128; displayed128 is
correct. Updated ONLY the whole App English and corresponding View ENKO refusal
goldens to128, preserving FSI/PDI and translation/cause/success
clearing. No policy capacity, FTL wording/vars or formatter changed. Kit/View
suites were not reached after App failed; no pass is inferred for them.

Announced base10 3e7820e3 is merged before the next narrow run. The only code
conflict combines the new checked diff module with existing Find exports and
ONE CodeEditor composition registration; constructor callbacks/editability
remain intact. W10 resident adapters and protocol25 values remain within their
own announced scope. Product main still does not contain S2's held sidebar
policy/caller: that exact source and its two keyboard/focus tests live only in
the separate authorized sidebar-check worktree pending8captures+leadunion.


## Isolated sidebar receiving evidence and keyboard producer

The corrected sidebar policy remains outside this product tree. Its isolated
receiving source d629c293 contains the exact owner policy/correction, this
package's sole ToggleSidebars caller, staged-attribute removal and two real-key
regressions. Run wp-commands-narrow-20261115T182847Z-d629c293 passed App/View/Kit
all-target strict checks. Run wp-commands-20261115T182847Z-d629c293 executed
App154/0/2, Kit235/0/0, View library1068/0/0 and integrations129/1/0: total
1586/1/2. Both new owner-intent/focus-exit/off-workspace cases and the corrected
600-width policy cases passed. The only failure was the unchanged page find
wall-clock assertion: cold2.298131175s against2s, with cargo-step load45.52 at
entry and46.93 at completion. This is not a green whole suite. A single named
case replay on the same source is pending under the coordinator's replay rule;
no timing threshold is changed here.

The coordinator has now compared and cleared the corrected owner's captures.
Main still waits for an announced green shared line containing that correction
before adopting the caller union. Narrow checks do not replace this condition.

A separate keyboard mechanics producer is prepared from the clean base10.
It includes the canonical registry/engine/prefs grammar, native help/rebinding,
callback dispatcher, actual kit composition/overlay boundary and only the old
palette's local-key/weak-close-focus integration. New palette pages, Messages
mode/navigation and unfinished git/file/sidebar/desktop effects are excluded.
The old base has17 actions while the canonical request requires the full typed
context and76 reference definitions; copying only dispatch.rs would not be a
self-contained API. This extraction is source preparation, with receiving
compile and actual GPUI evidence still owed. It is not a second keyboard
implementation or a completed F4 package.


## Announced base13 and sidebar caller adoption

The product tree now contains announced base13 and its reviewed corrected
sidebar policy. Following the coordinator's capture clearance and the lead's
green base12 announcement, the exact isolated caller and two focus-exit tests
are adopted without importing a moving sidebar branch. ToggleSidebars delegates
to the existing sidebar owner; only a handled action exits focus mode through
W12's existing adapter. The staged dead-code expectation is removed together
with its real production caller. No geometry, default binding or second key
listener is added by this adoption. Current-union checks remain owed.

The original isolated test log remains1586/1/2. The coordinator-authorized
single-case replay on the same d629 source passed1/0/0 under run
wp-commands-20261115T184803Z-d629c293. Cargo-step load was36.68 on entry and33.47
at completion. Both original and replay logs are retained; only the named
wall-clock case is closed by that replay. The deterministic find-work correction
subsequently arrived through the announced shared line rather than a private
threshold edit. No full-suite result is rewritten as green.

The keyboard-only extraction's first two receiving jobs both stopped at the
same missing crate visibility on the shared whitespace predicate, before any
App/Kit/View test executed. Its fix changes one visibility token and preserves
the existing ECMAScript predicate, rankings and marks. That extraction is still
not an independently checked producer or a full F4 package handoff.


[Decision Log: ready root actions share the dispatcher]
- Purpose: the reference root query-only toggle-left-sidebar item has an actual
  native owner and must use the same post-handled focus-mode exit as its key.
- Constraint: direct palette callbacks bypassed that common dispatcher behavior;
  manufacturing a keyboard event would mix physical input and typed intent.
- Alternatives: another sidebar toggle callback, synthetic keypress dispatch,
  or a parallel action/default table.
- Choice: the existing source callback closes the palette and calls a narrow
  window-local sidebar entry. It asks the same current context/admission helper
  and ordered dispatcher; only chord matching is skipped. No general palette
  shortcut variant can resolve a resource-scoped row against a later workspace.
- Why: the core owner remains the only implementation, with the existing W12
  post-handled exit. Actual popup context, recording and composition still deny
  a command. Compact pointer actions are independent of global-key availability.
- Effects: root toggle-left-sidebar is query-only, group0/rank8, using the
  existing PanelLeft glyph, canonical translated action label and effective
  hint resolver. No new binding or unowned NewWorkspace fallback is added.

A real-frame query/Enter regression and the actual-menu/capture fixture exercise
this typed route. They are written but not executed yet; in particular they do
not establish retained-hidden-opener visibility or OS input behavior.


Availability re-audit confirms that the existing snapshot projection already
filters the registered Help item using the actual keyboard-availability value.
No second registration predicate is added. A real resize regression now checks
600,599,390,600 widths while the pointer-opened palette stays open: Help must
follow the shared boundary and return when wide again. Execution remains owed.


The ready root Settings, Help and left-sidebar items now carry the exact
reference keyword inventories, including configuration/hotkeys/panel/workspaces.
Settings uses the existing open_settings(None) owner entry instead of pinning
a legacy section target; the owner still decides its default presentation.
A real query/projection regression covers those hidden aliases. Labels, ranks
and availability still come from their existing typed models. No missing
feature's route or source is manufactured by these corrections.


The first base13 main strict/test selection stopped in the new test fixtures:
sidebar_open/sidebar_shown are existing owner query methods, not raw fields.
The fixtures now call those methods; no visibility or sidebar state was added.
Strict also identified two redundant px qualifications and a redundant method
closure, now removed. These compile failures are retained as failures; none of
the new main regressions is claimed executed by those jobs.

Root-action icon limitation remains explicit: the reference root PanelLeft,
Settings and Keyboard helpers use stroke2.2, while their existing shared native
roles retain published1.5 vectors. The shared glyph owner recorded this future
role decision separately from the isolated new Import role. This package does
not modify shared vectors, add an arbitrary stroke API or claim those icons'
first-use parity before the owner/V1 decision and actual native verification.


[Decision Log: typed rebinding operation]
- Purpose: close V4 row295's T1/T9 finding without changing preference behavior.
- Constraint: the public reset boolean and independent optional replacement
  permitted a contradictory Reset-plus-keys state that silently ignored keys.
- Choice: one closed Edit::{Replace(&KeySequence),Clear,Reset} value crosses the
  view/app boundary; menu RowAct variants map directly to named operations.
- Why: impossible combinations cannot be constructed, and the existing shared
  migration/difference transaction still owns all preference mutations.
- Effects: preserve legacy alias migration, unassignment/default reset semantics,
  alternatives, bounds and every prior assertion. A pure sequential operation
  case and real menu Clear/Reset regression check the behavioral distinction.
  Changed View/App narrow checks are recorded with the same-row correction.


## Owned Import root contribution

W2's checked opener arrived in the announced line. The isolated shared Import
role96e14842 plus docs0e8e6a17 then received V1 source/Kit narrow and coordinator
source-sheet comparison approval. Only those five owned paths are merged here,
retaining the complete receiving inventory prefix and all existing vectors.

The import-session source uses group0/rank2 and import/session/terminal aliases,
with words.history().title(), Glyph::Import and no keyboard binding. It shares
open/close/language registration and generation cleanup with root Help. The
existing callback dispatcher closes the center before calling open_import; it
adds no host resolver, import operation, popup or automatic session opener.
The shared command row already paints leading vectors in foreground_muted,
independent of its parent hover foreground; no local color override is added.

An actual owner-modal regression replaces the obsolete first-root Settings
expectation: once the rank2 Import action exists, first Enter must open that
owned modal while preserving draft route, host selection and unsent input.
Native first-use capture and the receiving source checks remain separate owed
evidence; library/source-sheet agreement alone is not native UI/OS approval.


## Import receiving evidence (2026-11-16)

At exact f49784f885beda13fe8437ebee1dd3bc4b2cb4e7 on announced base13.1, run
wp-commands-narrow-20261116T015726Z-f49784f8 recompiled App and Kit and passed
all-target Clippy with warnings denied. Run
wp-commands-20261116T015726Z-f49784f8 recompiled the actual App binary and Kit
library and executed the import-filter selection: App4/0/0 plus Kit2/0/0,
with421 filtered cases. It exercised first-root Enter opening the sole Import
modal, retired registration rejection, successful receipt preserving the draft,
and retained owner callbacks refusing a scope ABA. Kit checked the three-path
2.2-stroke asset and fractional-scale shape landmarks. The separate frozen
inventory-prefix case was filtered out; its executed evidence remains the
producer's full Kit suite, with old receiving vectors source-preserved.

This is narrow receiving evidence, not the whole F4 gate or native glyph
first-use approval. The first-use capture must use the approved protocol25
root fleet. Export preparation found that the shared exporter still selects
the older protocol23 root set despite returning success. That common-tool
boundary is held with its owner; no old-root export or first-use capture is
reported as current evidence.


[Decision Log: stage Git contribution until owned effect admission]
- Purpose: a new palette entry must not broaden an inherited, unpassed Git write
  path into a claim of current original-connection/root/ticket authority.
- Constraint: the old captured UI generation/session/PullEntry snapshot and live
  selectable item check are presentation fences; G1 confirms no checked write
  owner packet is available on the announced line.
- Alternatives: infer readiness from alias/status equality, duplicate a lease or
  write policy in the palette, or leave a live row with a source-only caveat.
- Choice: stage only the F4 Git module, retained state and six registration/use
  sites under cfg(test). The existing contribution preparation stays compilable
  with its tests; normal product builds do not install workspace-git.
- Why: G1 explicitly approved this exact boundary. Existing toolbar/menu callers
  still consume its adapter, so staging requires no owner edits or lint allowance.
- Effects: no new palette Git effects until G1 supplies the checked scoped-write
  producer and the receiving union passes. Existing inherited Git consumers remain
  a separately unpassed dependency; no copied RPC, identity cache or policy.


[Decision Log: layout row search fields and shared glyph roles]
- Purpose: the reference's hidden aliases must work independently of translated
  labels. A single raw command id omitted vertical/horizontal, switch/navigate
  and the single-pane zen/distraction/fullscreen aliases.
- Existing assets: the current layout source already owns captured W12 contexts;
  the announced Kit inventory already contains Columns2, Rows2, X and ArrowLeft.
- Choice: retain one source and give its local presentation spec named fields for
  action, rank, capability, glyph and canonical keyword slices. This extends the
  existing model rather than adding a matcher, listener or glyph renderer.
- Source: pinned Paseo1c00 workspace-contributions.ts264-290,398-502,684-691 and
  workspace-registration.ts69-77. Existing passive tab chooser stays a documented
  native extension; every actual effect still uses run_layout_action_at.
- Effects: reuse Columns2/Rows2 for split directions, X for tab/pane close and
  ArrowLeft for previous tab. Other missing exact roles (ArrowRight, Focus, Move,
  ArrowDownToLine) are reserved with the sole shared glyph owner, not fabricated
  or claimed visually complete here. No existing vector or stroke is changed.
- Verification: a new actual-frame Korean query regression types hidden English
  aliases at split and compact widths; execution is still owed at this checkpoint.
  The existing stale-workspace action case moves unchanged to the same focused
  test child module, reducing the oversized parent.

Git staging exact05fdbeaa6 passed App all-target strict checks in run
wp-commands-narrow-20261116T021433Z-05fdbeaa. This compiles both the product
configuration without F4 Git registration and the staged test configuration.
No new write-owner readiness or inherited toolbar approval follows from it.


Layout alias receiving evidence at exact d31181c47754106050a8d4e0fbc311a7bc525f2f:
run wp-commands-narrow-20261116T021940Z-d31181c4 passed App all-target Clippy
with warnings denied. Run wp-commands-20261116T021940Z-d31181c4 recompiled the
actual App binary and passed both layout_actions cases2/0/0,177filtered: Korean
physical-field input found canonical hidden aliases at1280 and599 widths, and
a captured close remained refused after the selected workspace changed.
No missing glyph-role visual approval or new resource authority is inferred.

The common exporter owner has repaired the old-root selection boundary with
the explicit --shared-root-fleet base12 option. The selected root is protocol25
and schema21, independently pinned to its own b5b46357 commit and full manifest
and checksum digests; same_commit remains false relative to the app. One bounded
native-build app export is submitted on d31181c4 through that approved route.
Its first-use originals, inspection and coordinator comparison remain owed.
This bounded build is separate from the final whole F4 gate/capture matrix.


[Decision Log: one global UI shortcut admission predicate]
- Purpose: follow pinned _layout.tsx398/416 and the coordinator's explicit book
  ruling; a native always-mounted titlebar does not authorize global shortcuts.
- Existing assets: Shell.loaded records installation of the first successful
  Fleet snapshot. Link refresh first awaits saved_entries; book failures emit
  failure and return, while configured offline entries remain in the successful
  snapshot. No selected-session, online, welcome or resource lease is needed.
- Choice: expose client_book_read over that existing fact and one Shell predicate
  global_shortcuts_enabled = Settings OR (book read once AND nonempty entries).
  The listener, dispatcher availability/activation and badge eligibility reuse
  it; viewport600/mobile policy remains independent. A single Fleet event hook
  retires pending prefixes/capture/badges when the global UI becomes disabled.
- Alternatives rejected: infer readiness from selected session or successful
  welcome, duplicate S2/W15 policy, or add a second book cache/event publisher.
- Effects: an offline configured entry enables UI shortcuts, an empty/unread book
  disables them outside Settings, and a custom feature handler cannot bypass
  admission. Onboarding chrome presentation stays W15's separate task.
- Verification: existing keyboard harnesses now explicitly seed a configured
  offline book, preserving their enabled-input intent. New real-key cases cover
  unread/empty/offline and Settings; direct owner dispatch and book-loss chord
  retirement have their own regressions. They are not yet executed.


## Announced base14.1 receiving merge

The lead announced b54abbdc after its1755/0/2 native gate, wasm check and
coordinator picker comparison. Its shared keyboard producer overlaps this
full F4 tree. Resolution retains the current single command center, nested
page Back and Messages lifetime, root Import/source cleanup, typed menu edits,
sidebar owner/focus-exit caller and new book admission. The producer's older
palette/keyboard.rs is omitted because its two methods already live in the
current palette/navigation owners; no duplicate key controller is retained.

Incoming desktop stand-in dispatch regression joins existing rebinding/sidebar
fixtures. CodeEditor preserves both its existing find module and incoming
revision tests, with one composition registration. Automatic field merges keep
FormText, compact textarea presentation, FieldId/captured callbacks, editability
and FormMinimum alongside one line/lines composition registration. This is
source resolution only; changed receiving compilation/execution remains owed.


Global book admission must not steal keys owned by a pointer-opened local
control. The full command center's native empty-query Backspace runs before the
input toolkit's delete action; disabling global matching now retains that
local operation after the existing overlay/IME checks, just as compact windows
do. A real-field regression covers an empty book with a nested Theme page.
This is a full F4 consumer seam; the smaller shared producer's older palette
has no nested-page Back operation. Neither adds another global listener.


Announced14.1 main union exact5ebb31b0 executed App196/0/2, Kit272/0/0,
View1128/0/0 and integrations/examples142/0/0: total1738/0/2 in run
wp-commands-20261116T024729Z-5ebb31b0. The three new book admission cases,
desktop stand-in, sidebar focus exit, code-editor composition and incoming
revision regression explicitly passed. This precedes the added local-page Back
case. Its paired strict run wp-commands-narrow-20261116T024729Z-5ebb31b0 failed
only a new documentation backtick around saved_entries in admission.rs. That
comment is corrected without a lint allowance; current App checks remain owed.
No whole gate or first-use capture result is implied by this narrow suite.


At exact7be03b612a4c149e8311aedc4c8f51eab2739c38, receiving run
wp-commands-narrow-20261116T025851Z-7be03b61 passed all four native crates'
all-target Clippy with warnings denied. Run
wp-commands-20261116T025851Z-7be03b61 recompiled the App binary and passed
197/0/2, including the new empty-book local-page Back case and all three book
admission cases. Unchanged Kit/View execution remains the earlier1738 suite,
with its exact limits retained. There is no final whole F4 gate or UI/OS claim.

The earlier d311 first-use export never took a slot. After the lead required
announced14.1 for first use, its queued-only cancellation was verified terminal
143 and raw records retained. A single replacement native-build export is now
registered as wp-commands-20261116T030518Z-7be03b61, ordinary lane, explicit
protocol25 base12 fleet. No app/display/fake-fleet process or image exists yet.
The shared book fact is also assembled as a bounded nine-path producer on
announced14.1, excluding the moving full F4 tree; portable checks remain owed
before W15/lead consumption.


[Decision Log: hold only new Messages product exposure]
- Purpose: W1 confirms the announced line still has staged residents and an
  alias-only final Open. Reply guards and prepared destinations do not supply
  original-context destination admission.
- Choice: stage only the new root contribution, parent mode selector and
  PaletteTarget::Messages transition. Product dispatch of a retained Messages
  target uses the existing unavailable feedback and remains in Commands.
- Owner boundary: W1 explicitly permits literal cfg(test) attributes on its
  existing palette_mode and scope functions plus the scope-only segmented
  import. Their bodies, search/controller state and result renderer are preserved
  byte-for-byte; enum and one window-retained Searching object remain unchanged.
- Why: enabling a prepared alias navigation path would broaden an unaccepted
  owner effect. A second resolver, controller, dead-code allowance or duplicated
  renderer would hide that gap.
- Effects: preparation fixtures remain compilable while product entry points
  await the checked owner carrier. Existing SharedString imports remain product
  imports because this receiving body uses them in result IDs and messages too.
  Current receiving strict checks are owed; this is not full A3 acceptance.


## Announced base14.3 and C5 shown-source receiving boundary

Before the next remote check, merge announced67637eb6 (14.3, superseding14.2).
The canonical C5 follow/session-read functions now live in source_follow; their
old inline duplicates are removed. Preserve its RootReceipt, single ticket
allocator, conservative legacy-viewer hold and code_tab source invalidation.
Retain both the W12 Explorer exports and C5's literal test-staged shown-context
exports, with pure FileQuery/Explorer modules alongside canonical file_scope.
W13 explicitly confirmed this bounded resolution. No alias flush/retarget is
restored, moving C5 branch imported or F4 Files/opener effect enabled. The old
IO fixture adds receipt=None explicitly; it gains no original-context receipt.
The incoming CodeSource web arm stays exactly as announced rather than taking
an unannounced formatting predecessor. Receiving checks remain owed.

V4 row312 separately passed only the portable book fact/global UI gate source
c7eb3b1f1 on base14.1; native bytes equal executed159f6856. R37 App strict0 and
R29 actual AppBIN19/0/0 were independently verified. That shared source is handed
to the lead; consumer adoption waits an announced receiving green. This is not
final F4, W15 chrome or live node/OS/persistence approval.


The first14.3 receiving strict run R38 stopped before App tests: the automatic
union had declared/initialized the existing FileTickets allocator twice, and
the incoming retained-viewer test lacked the older receiving Viewer's three
extra inert fields. The duplicate declaration/initializer is removed, preserving
one allocator shared by old and canonical readers. The test adds reading=None,
workspace_dir=None and intelligence=Idle without changing its assertions or
creating a connection receipt. W13's canonical follow/body policy stays intact.
Current receiving checks remain owed; R38 is retained as a compilation failure.
