# Milestone 12, G1: native git and pull requests

## Boundary and existing assets

Extends `spikes/native/view/src/workspace/` (track F's model); it does
not introduce another workspace model. The existing `git_quick` owns
Commit/Push/Pull/Fetch selection and disabled reasons. Existing
`text/workspace` owns English and Korean refusal wording, paths and
hook/remote output. `link::workspace::{GitAsk, GitWrite, GitWritten}`
already covers the node's git calls, so those calls are reused.

References: Paseo's branch switcher and git action split button first;
GOAT's branch toolbar and git action control; Vessel's web workspace
dialogs and `data/workspace/{panel-model,words}.test.ts` for behavior.
Their source is read for behavior and layout; no reference code or
assets are copied.

## Slice 1: action preparation

`view::workspace::git` owns preparation before a node call:

- `form`: the commit's changed/staged file counts and initial scope,
  trimmed message parsed by the wire constructor, and the stage/unstage
  actions on each side of a status row.
- `branch`: create from HEAD, rename, and switch a remote-tracking ref
  to a local branch starting at that exact ref. A ref that cannot name a
  local branch is a typed validation failure.
- `discard`: the confirmation's exact root and path and whether the
  action restores HEAD, deletes an added file, or deletes an untracked
  file. Its request is made only by consuming the decision on confirm.
- `stash`: apply, pop and drop retain both the listed index and commit;
  concurrent stash changes are left to the node's `stash_moved` refusal.
- `state`: a lifetime-long request-key issuer, correlated reads that
  reject old and duplicate replies, and retained content during refresh
  that cannot authorize a write until the fresh read answers.
- `written`: exhaustive interpretation of the existing wire outcomes
  and one outstanding write, protected against double clicks and stale
  completion. Refusals preserve their typed paths and hook output.
- `text/git`: every new preparation failure in English and Korean,
  with a localized explanation and next step above exact diagnostic
  detail. Branch, upstream and file counts are full sentences; truncated
  lists say that their counts are incomplete. Long numeric counts are
  grouped for readability. `Display` is reserved for logs.

Data flow: current wire status/list -> pure preparation -> existing
link request -> node outcome -> workspace refusal words. Screens and
their kit controls follow in the next slice. There is no new dependency,
UI toolkit type, I/O, or wire addition in this slice.

[Decision Log]
- Purpose and intent: keep git decisions independent of drawing and
  preserve the page's tested behavior on every native surface.
- Existing implementation and constraints: F already provides git
  selection, words and typed calls; the node owns git validation.
- Alternatives: repeat those APIs in a separate git model or place
  request preparation directly in GPUI callbacks.
- Choice: add small pure modules under the existing workspace model and
  use the wire's own constructors for typed text.
- Why: one validation contract and no new crate dependency; the screen
  cannot accidentally implement another interpretation of git names.
- Consequences: preparation still can fail when a listed remote ref
  cannot become a local branch name; the screen must show that failure.

Deliberate behavior difference from the page: a listed ref with a remote
prefix inconsistent with its `remote` is a typed `RemoteMismatch`, rather
than stripping a prefix by length and potentially checking out a
different local name. A normal remote branch has the same request on
both surfaces; malformed replies must never select another branch.

## Contract gap: all-project pull request page

The requested page is every open pull request of every machine project.
Current `forge::ListPullRequests {}` and `PullRequestList` contain only
watched live sessions. The node's `forge::watcher::list` iterates those
sessions, truncating at its watch limit. It cannot enumerate a project's
PRs without a watched session. A watched-session list must not be
presented as the complete all-project list. The coordinator narrowed
G1 to a page explicitly headed "The pull requests of this machine's
sessions" (Korean: "이 머신 세션의 풀 리퀘스트"). The all-project gap is
reserved for a node track; G1 does not change wire or node code. A future
node call would be `pr.project_list { project, state: open, cursor?,
limit? } -> ProjectPullRequestPage { entries, next? }`, with repository
identity rather than a mandatory live-session id; this is a proposed
contract, not an existing or callable method.

The current `git.commit` has no amend option. Neither reference's git
action controls inspected for this slice offers amend, so an amend
control is not introduced or emulated.

## Validation and remaining work

The native workspace's three-command gate passed for this slice:
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test` (333 tests passed, none failed or ignored). The gate is the
whole native workspace on its pinned toolchain; no main-workspace code
changed, and no main-workspace gate or platform CI is claimed.
Tests cover the web dialog inputs, both halves of a mixed
status row, all file states, message limits, invalid names, exact remote
start refs, each discard effect, and stash identity retention.
The references' own `git/policy.test.ts` (Paseo) and
`GitActionsControl.logic.test.ts` (GOAT) provide three additional
git-only cases: pull with two incoming commits, push on an unpushed
worktree branch, and commit with local changes while two commits behind.
Counts map to `GitStatus` distances and file state; expected English
labels and availability are unchanged. PR/archive/local-merge promotion
from those suites is not covered by this slice. The reference branch
switcher's directory-versus-workspace-id case belongs to the link/UI
wiring; Vessel's wire uses typed `WorkspaceRoot` rather than a directory
string, and that integration remains to be checked.

Whole-sentence tests cover every new preparation reason and branch state
in both languages, along with singular, empty, diverged, large and
truncated count cases. Parser rule data and unsupported characters are
retained as detail; no new error's `Display` becomes UI guidance.

No screen is drawn by slice 1, so no capture or visual claim applies.
Git UI, local fake-fleet operation checks, the requested shape additions,
and PR UI are not delivered by this slice. Request keys are app-side
only and never change a node request. No service, file permissions,
preview unit or production account is changed; these are development
view-model changes only.

## Reuse by the code panel

`Reading<T>` and `RequestKeys` are generic despite their current home
under `workspace::git`. They stay there for now, as the first consumer's
tested implementation. Track F should reuse them for its code panel's
late-reply handling, then lift them to a shared view module when that
seam is agreed; a second key issuer or read-state implementation would
reintroduce two interpretations of the same rule.

The slice's review also removed phonetic particles after interpolated
Korean branch names and made a small limit read as "255 bytes" /
"255바이트", while a larger limit keeps its readable "64 KB" form.
Both sizes and the complete Korean sentences have tests.

## Reviewed controls integrated before the screen

The lead's branch through `777b2da` was merged before screen work, which
brings its discrete slider and shared spinner (`103dd6d`) rather than
another activity indicator. The separately reviewed badge control
(`840a941`) was cherry-picked unchanged for future PR state labels. Its
own control notes describe the semantic signals and the shared adapter;
G1 does not introduce another badge.

The integration also includes the three small follow-ups from the view
slice's review above. No git screen or split-button registration is
included in this integration.

The pinned native workspace gate passed on this merged tree: formatting,
Clippy with warnings denied, and all native tests. This verifies the
integration; no screen, platform CI or screenshot is claimed by it.

## Shared split button (control slice)

`kit/src/split_button.rs` is the one shared split action, after Paseo's
`git/actions-split-button.tsx`. Its joined outline is 26 points tall with
a pointer and 44 on touch, at radius 6; the primary inset is 8/12,
content gap 4/8, caret width equals the height, and a one-point divider
uses `borderAccent`. Hover uses `surface2`, disabled primary opacity is
60%, and a busy primary uses the lead's shared spinner. Busy and disabled
states have no primary handler; the caret still opens the other actions,
as in Paseo. Labels and callbacks belong to each area; no git policy or
localized strings live in the control.

The popup and picker were read before adding placement. `popup.rs`
provides surfaces and geometry, not anchoring. `picker.rs` already uses
GPUI's `Anchor` to stay on the caller's chosen side; `menu.rs` previously
used its default top-left anchor. The new `menu_right` entry point uses
that same primitive with `TopRight`, without changing `MenuState::open`
or existing menu callers. Both entry points share the entire menu
renderer, including keyboard handling, window-margin snapping, and row
wrapping. The reason for aligning to the caret's right edge is recorded
beside the entry point. No second popup or positioning engine is added.

Paseo has no split-button-specific test file. Its
`menu/menu-overlay.browser.test.tsx` was read: it checks that a growing
surface keeps its anchor and can widen. Native placement remains
computed by GPUI on every layout, rather than a snapshotted rectangle;
actual growing/wrapping menu captures belong to the following screen
slice. The two control tests cover the three primary states and the
pointer/touch reference geometry. Menu navigation is unchanged.

Touch targets are 44 points as the native README and ADR 0036 require;
Paseo’s header control is 32 on touch (`control-geometry.ts`) and 26 with
a pointer. The pointer size is retained exactly; touch is deliberately
larger to honor the app’s target-size rule.

This slice draws no area, so there is no screenshot or runtime placement
claim. Its API difference is an explicit reusable control instead
of Paseo's git-specific props; the area retains policy and menu content.

The final control tree passed the pinned native workspace's exact gate:
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test` (339 tests passed, none failed or ignored). No main-workspace
code, dependency, service, permission or running account changed. The
control slice has no app registration or screenshot claim.

The lead's next small shared-control correction through `7945408` was
merged before the git screen: the effort slider's last step remains at
the top, and other agents' lists do not gain options invented by Vessel.
The source changes are the lead's unchanged integration. The exact
native workspace gate was rerun on this merged tree before committing.

## Code panel and shared Alert integrated

Track F's code panel through `0495cf7` is merged before the git slot is
filled. It owns the Changes/Files frame and its session/sidebar hooks;
G1 has not adapted those sources in this integration. The shared Alert
from `85cc00b` is also applied unchanged, including its pure notice
vocabulary and the one app adapter. Refusals and failed reads in the git
screen will use that control, with Git's exact detail beneath it.

The merge keeps both output/menu exports, both README control entries,
and every embedded glyph from F, the lead and Alert; the asset table is
45 entries on this integration tree. The following screen slice takes
F's refresh seam and file split before filling its Changes-head slot.

The pinned native workspace gate passed on this integration: formatting,
Clippy with warnings denied, and 341 tests (none failed or ignored).
No G1 screen registration or captures are included; its in-progress
changes are kept outside this integration. No main-workspace source,
service, production account or permission changed.
## Shared spacing scale

`kit::theme::spacing` exports all sixteen steps of Paseo's `SPACING`
table, including fractional keys. The constant names retain the keys;
the values are logical points. An exact-value test checks the complete
table. The module's Decision Log records why a shared complete scale
replaces private inset and gap literals. This slice registers the scale
and documents it; consumption belongs to the git screen slice. Existing
areas retain their geometry. There is no difference from the reference
scale and no screen change to capture in this slice.

## Shared dialog overflow

A real hook refusal with 120 lines exposed that the dialog card clipped
its actions when its content exceeded the existing maximum height.
`kit/dialog.rs` now gives only the body an automatic flex basis, a zero
minimum height and vertical scrolling. The automatic basis keeps short
dialogs at their natural height; a zero basis would change their fit.
The header and footer cannot shrink. The card's dimensions, surfaces,
callers and maximum height are unchanged. A GPUI layout regression uses
one-line and 120-line bodies at 1280 and 390 and checks that actions keep
their height and remain inside the window.

The existing session-kill and archive confirmations are the fitting
cases: both were captured before the change, dark and light, at 1280
and 390. Those checks use the fake node and dismiss each decision; they
do not stop or archive a session. The following captures compare those
same dialogs and exercise the real hook refusal with its actions.

The after checks are complete and accepted: both fitting confirmations
retain their size and position at both widths in both themes. Comparing
their opaque interiors gives zero changed pixels in all eight cases.
The real hook refusal's body/output can be scrolled while its title and
both footer actions remain fully visible at 1280 and 390, dark and light.

## Linux/X11 input-method finding (existing composer)

Before adding a multiline form field, the existing composer was checked
under an isolated Xvfb display with a private `fcitx5 -D`, Hangul input
method, configuration/home/cache directories and session D-Bus. Both
clients used `LANG=en_US.UTF-8` and `XMODIFIERS=@im=fcitx`. No session
prompt was submitted. A separate small Xlib control client opened the
input method, created an input context with
`XIMPreeditNothing | XIMStatusNothing`, selected its filter events and
called `Xutf8LookupString` after `XFilterEvent`. Typing `gksrmf` and then
Space committed `한`, `글`, and a space in that client. The unchanged
Vessel composer on the same display entered `gksrmf` instead. Both
results were inspected. The diagnostic clients and private input-method
processes were stopped by their recorded process ids.

The pinned GPUI X11 client creates an input context with
`PREEDIT_CALLBACKS` without a status style. That is a possible cause,
not a proved cause or fix. The coordinator assigned this existing
composer/platform defect to the lead. G1 does not change the platform
or editing engine. Live Hangul composition through an input method was
not shown for the existing composer on this display; the new form
inherits that same engine and needs the same qualification. Its own
marked-text regression checks the shared input-handler boundary.

## Shared multiline form field

`TextField::form_multiline` shares the composer's input-state factory,
input handler, composition, selection and undo. Its typed presentation
uses the one-line form field's existing shell, including its border-only
focus treatment. The composer and existing one-line callers keep their
entry points. GOAT's commit dialog uses its small textarea: the outer
minimum is 68 points (80 on touch), with 9/3-point insets and a 256-point
cap. At the kit's 20-point line height the native engine grows from three
to twelve complete lines, then scrolls within that cap. The form uses
Enter for a new line and Mod+Enter to submit, independently of the chat
composer's send shortcut. The Decision Log records the shared-engine
constraint and the deliberate whole-line cap.

The kit previously had only send-shortcut tests, with no marked-text
composition test for its composer. The new GPUI regressions exercise
both public constructors through the same mark-and-replace calls,
checking partial Hangul marks, committed text and no submission. Separate
layout checks grow a form through 1, 6, 20 and 30 lines at both densities,
keep the complete text and require the last line to scroll into view.
A keyboard check covers Enter and Mod+Enter. These handler/layout checks
do not establish live platform input-method support: live Hangul through
an input method was not shown on this display for either presentation.

The new form notifies its owner after each edit so a growing field updates
the surrounding dialog and its submit availability. A pointer regression
blurs then clicks the real input inside and outside the kit dialog at
both widths, checks that typing reaches the input, and checks that the
owner renders the changed value.

## Shared toolbar label trigger

`kit::toolbar_label::trigger` follows Paseo’s
`components/ui/toolbar-label-trigger.tsx`: a label without a leading glyph,
a 12-point caret, 4-point insets and gap, radius 6, muted text at rest and
foreground text on interactionHighlight when hovered, pressed or open.
It reuses the kit’s 12-point Meta type step. Its disabled state preserves
the frame and has no interaction; callers attach no disabled click.
The composer’s round pill is deliberately unchanged. The default choice
is a single line with the reference’s truncation; a typed wrapping option
is the requested departure for long branch names in the Changes head.
Touch takes the shared TOUCH_TARGET minimum instead of changing text size.
The six-argument frame leaves content, state, layout and tokens visible.

## Branch picker’s shared leading size

[Decision Log]
- Purpose: match the 14-point branch glyph supplied by Paseo’s
  `components/branch-switcher.tsx` renderOption without an area-owned row.
- Constraint: the kit’s existing `Leading::Glyph` is 16 points everywhere.
- Alternatives: resize every picker glyph, or draw another branch row.
- Choice: backwards-compatible `Leading::SmallGlyph`, preserving every
  existing caller and the rest of the combobox’s layout and behaviour.
- Why: the reference chooses this size explicitly; a generic picker’s
  existing size must not change as a side effect of one new caller.
- Cost: one additional typed leading form; no additional surface or engine.

## Exact paths in code tooltips

[Decision Log]
- Purpose: the full form of a shortened path/hash stays in its own fixed-width
  element, as GOAT’s `components/StartTruncatedPath.tsx` code tooltip does.
- Constraint: existing tooltips use body text and must keep their presentation.
- Alternatives: put a full path in body text, or draw another popup in an area.
- Choice: `Tooltips::on_code` shares the same attach/render path and every
  chrome/placement value, with typed content selecting its own face/metrics.
- Why: existing `on` callers stay unchanged; one tooltip surface serves both.
- Cost: touch still has no tooltip, as before. A narrow pointer capture checks
  code wrapping at 390; it does not claim that touch gains hover interaction.

The path’s inline middle ellipsis uses the pinned GPUI’s existing
`text_ellipsis_middle`, not a new truncation algorithm. GOAT’s RTL/bdi
`StartTruncatedPath` instead shortens the start. Keeping both the initial
project context and final filename/extension follows the coordinator’s
explicit middle-truncation instruction; the entire path remains in code.

## GOAT’s shared commit summary frame

`kit::form_summary::{summary, files}` follows the branch/files group and
its file-list inset in GOAT’s `components/GitActionsControl.tsx:1914–2030`.
The summary has radius12, padding/gap12 and zinc25/light (index.css:273,
neutral OKLCH .992 converts to nearest 8-bit sRGB #fcfcfc), white3.5%/dark.
The inset is radius8 and 176 points tall, white/light and white2.5%/dark.
Both static edges are black5%/white5%. A passive absolute child
paints border1 entirely outside the content box, with outer radii13/9 and
inner radii12/8. It consumes no layout space and never changes with focus.
A native drop shadow was rejected: GPUI paints it under translucent fill
too and does not expand its corner radius, changing the reference’s paint. Settings cards and
all of their callers stay untouched. Exact paint/geometry and a real GPUI
layout test check that the edge does not steal a pixel from either frame.
The existing virtual list supplies content; no new text/list engine.

The code tooltip’s literal GOAT typography (`ui/tooltip.tsx`) is 11 points
with leading-relaxed (17.875), both scaled by the independent code-size
setting. Ordinary `on` remains 14 / 19.6 on the interface scale. A returned
review caught the first implementation overriding the code line with an
interface-scaled line; unequal UI14/code18 now has an explicit regression
without mutating global font settings during parallel tests. The native
control deliberately keeps the existing shared Paseo chrome: maximum280
(rather than GOAT max-w-120 /480), radius12, insets4/8 and existing placement.
The pinned engine wraps an unbroken path within that width. This shares
one popup surface rather than silently claiming GOAT’s whole geometry.

## Git screen policies before drawing

[Decision Log]
- Purpose: keep screen availability, row identity and request feedback out of
  GPUI callbacks before wiring the toolbar and forms.
- Existing assets: reuse `git_quick`, the accepted request constructors,
  `Reading`, `RequestKeys`, `Writing`, the workspace refusal words and the
  existing viewer-local time policy in `devices::time`. No dependency is added.
- Alternatives: infer a pending action from later repository facts, authorize
  against the selected session instead of the shown code panel, or close a
  draft when its background read fails.
- Choice: typed menu/header/branch/scope models, exact shown-repository matching,
  a keyed pending intention and a retained commit-facts snapshot. Only a fresh
  ready read authorizes submission, including the keyboard submit path.
- Why: an old reply cannot move a spinner or notice to another action; a failed
  refresh cannot erase the edited form or authorize work on another repository.
- Cost: bounded repository/branch/file facts are retained while shown. Drawing
  and node integration are a later slice; this is not a finished Git screen.

The Changes head follows Paseo's `git/actions-split-button.tsx` and
`components/branch-switcher.tsx`: branch once, one short state in words, a base
only when different, full sentences in the tooltip and no duplicate file counts.
Detached commits are separate short/full code values, not body-face prose.
A clean synchronized repository has Paseo's menu-only shape; Fetch remains in
its menu. The existing page's quick-action, refusal, branch parsing and default
branch cases remain fixtures (`data/workspace/panel-model.test.ts`,
`words.test.ts`, `features/workspace/git-dialogs.tsx`); Paseo's `git/policy.test.ts`
and GOAT's `GitActionsControl.logic.test.ts` provide the additional policy cases.
The node has no branch-merge/archive operation in this scope; none is invented.

Branch choices follow Paseo's `hooks/use-branch-switcher.ts`: the current choice
keeps its check and closes without a write. All bounded returned branch records
and truncation facts remain in the model; no second 200-row cutoff is applied.
A branch failure after its picker closes cannot replace another form's notice.
A pending request retains its own intention, path or stash index plus commit;
its English/Korean caption is independent of Git's diagnostic text.

Per-file policy follows Paseo's `git/file-header.tsx`,
`components/file-actions-menu.tsx` and the uncommitted-only revert policy in
`git/diff-pane.tsx`. It accepts the code panel's actual shown machine/root,
returns no slot for a mismatch or Base, limits Staged/Unstaged to their own
index side, and offers full-file discard only in Uncommitted. Stage/unstage
are node-backed additions; a destructive request still consumes the prepared
exact decision. The slot's final layout and successful refresh need runtime
proof in the drawing slice.

The commit summary follows GOAT's `GitActionsControl.tsx` branch/files group;
per-file action pills do not belong there. Vessel's three-way `CommitStage`
choice is an explicit node-contract addition: “Changes to commit”, with
“Staged changes only”, “All tracked changes”, and “All changes, including new
files”, plus Korean counterparts. One value trigger/list avoids three unequal
wrapped pills on a narrow form. Page follow-up: its `commit-dialog.tsx` still
says “Stage first”; it must take these same words in its next pass.

Saved-entry refs and hashes remain separate code elements. Relative saved times
reuse the existing age policy, and exact tooltips use viewer-local seconds and
a named UTC offset. Every new pending/error/destructive sentence is tested whole
in both languages. This view-only slice adds no service or file permissions;
no live input-method proof or Git-screen capture is claimed.

A pre-report review also removed hashes from successful commit/pull prose:
`GitSuccessText` carries one whole localized message plus typed code facts
(created commit, or both pull endpoints). Exact 40/64-digit ids are tested
separately from the twelve-character code captions. Upstream-set/kept push
sentences are separate whole translation strings, not a conditional suffix.

Validation corrections before this slice was committed: the development-only
view test build first found ambiguous parsed-path types in two new tests;
explicit `RelPath` annotations corrected them. The first whole native clippy
run found an unnecessary explicit lifetime in `branch_failure`; it was elided,
with no lint exception. The next whole test run rejected a supposed 64-digit
success fixture that actually held 68 digits (`InvalidText::TooLong`, max64);
the fixture was corrected to 64, preserving the production validator. None of
these failing trees was committed. The final whole native gate is the delivery
check; development-only package tests are not represented as that gate.

## Integrating the current desktop base before drawing

The desktop lead's newer base brings its single-XIM-context Linux platform
patch, focused marked-text query, file-failure words, translation scaffolding
and reviewed shared glyphs. The merge preserves the accepted multiline form's
engine, shell and tests alongside the lead's `TextField::composing` query and
its three-field regression; neither input handler is replaced. Text registration
keeps both the Git exports and the lead's file/localization modules. README
keeps the already-reviewed code tooltip and summary-frame documentation. These
are overlap resolutions, not a new control or a platform patch made by G1.
Live Hangul composition still needs its own runtime evidence; an imported
platform fix or passing handler tests is not that evidence.


## Review corrections before the Git screen

V1 returned the view-only slice with three concrete findings; that slice was
not represented as having passed review. The selected scope's known eligibility
now follows the node's actual `git add --update` semantics: untracked-only
complete status cannot enable Tracked. One shared path rule supplies the path
preview, derived counts, scope choices and submit. Already-staged additions
remain eligible for Tracked. A partial status's zero is unknown, not proof of
an empty scope; its preview vocabulary distinguishes “Not listed” from “none”. Complete
empty, untracked-only (both wire spellings), mixed and staged cases exercise
both gates, with the existing English/Korean whole scope labels.

Empty, Failed and Loading reads have distinct availability. Only Loading says
work is in progress; Failed gives retry guidance and continues to stop automatic
polling. Whole sentences in both languages are tested against the same state
and polling decision, rather than checking only a caption in isolation.

`Writing` keeps a private-constructor `SuccessReceipt`: the exact accepted key,
original intention (including the pinned stash entry), and complete successful
outcome. Its existing `finish` result remains available to refresh the panel.
Save tracked, save with new files, Apply, Pop and Drop each have their own full
English/Korean outcome, with ref/hash in separate code facts. Apply is tested
with the unchanged returned list; Pop/Drop with an empty list. No operation is
guessed from that list. The legacy unlabelled tracker may only say a request
completed; visible writes use `begin_action` with their actual intention.
Paseo's `git/actions-store.ts` keeps success for 1000ms. The model's keyed reset
cannot clear a newer pending request or a later success; the renderer will own
only scheduling that same callback. Bounded outcome facts are retained for this
short phase while the original result is consumed for repository/list refresh.

Two additional pre-report regressions were caught: an arbitrary suffix could
hide the distinct upstream origin/other/feature for branch feature (only one
remote component may be stripped; the wire's RemoteName forbids slash), and an
older branch-list current flag could disagree with the header's newer status.
The picker now follows that same displayed status for its check/no-op and waits
for both reads before enabling a choice; the original complete branch record
is retained. These are covered by exact model and English/Korean label cases.
No screen, live input-method or node-write proof is claimed by this correction.

Validation: whole native fmt, clippy (all targets, warnings denied), and tests
passed: 506 runtime tests plus two compile-fail documentation tests. This same
frozen source also validates the independent MonoBody control, committed
separately; the unchanged gate is not repeated. Earlier compile attempts caught
BranchName/RefName comparisons (including three test constructors) and two
Markdown documentation lints; typed name comparisons and fixtures were corrected
without relaxing validation. No screen, node-write or live IME claim follows.


## Shared form path type step

[Decision Log]
- Purpose: reproduce GOAT's `GitActionsControl.tsx` text-sm summary inherited by
  its font-mono file rows: 14/20, normal.
- Existing constraint: the kit's Mono 12/20 is already used by paths/logs and
  must stay unchanged; UI and code preferences are independent.
- Alternatives: area font literals, or a UI-scaled label with a code family.
- Choice: `Type::MonoBody` adds 14/20 normal on the code scale and mono face;
  `TypeExt` continues to supply the existing fallback list.
- Why: both metrics follow the same independent setting, without changing old
  Mono callers or adding a second typography policy in the Git area.
- Cost: one public type step. No font asset, font loader or input engine changes.

The same production metrics resolver is tested with injected independent
UI14/code18 scales (Mono18/30, MonoBody21/30, ordinary Body14/21), without
mutating process-wide atomics in parallel tests. Default exact values and both
code families are checked. Actual long-path Git first use, both languages and
widths plus unequal preferences, remains required before visual approval.

Validation: the same whole native 506 runtime plus two documentation-test gate
as the preceding independent view correction; no unchanged-source rerun. Actual
Git first-use captures remain pending.

## Integrating protocol 12 and the embedded Light faces

The next desktop-base merge imports the lead's reviewed node/session line and
its protocol-12 event readers, the current conversation and machine-book work,
and the real Geist/Pretendard Light faces. No node or platform implementation is
changed by G1. The README merge retains both the machine-book architecture and
Git's existing policy/type-step notes. The font merge keeps both Light assets
and the independent MonoBody metrics/family rules; loading/fallbacks remain the
lead's. This integration does not prove live IME, title face selection or a new
Git screen. Those each require their own runtime evidence.

Validation: whole native fmt, all-target clippy with warnings denied, and tests
passed (533 runtime tests and two documentation tests). Only native checks are
reported here; imported root/node changes retain the lead's original provenance,
and no new root implementation or root test run is claimed by this merge.

## Local and remote branches with the same printed name

The next V1 review found that raw reference text is not branch identity:
a local branch literally named origin/topic can coexist with the remote-tracking
origin/topic. One shared local-only current predicate now supplies both the
selected check and the no-op. Every remote row still goes through switch_request,
so it prepares local topic/create=origin/topic and validates its declared remote
prefix. The regression compares the complete three-row projection (local,
remote, malformed-remote), the whole local Current and remote Switch outcomes,
and the exact typed RemoteMismatch. The original branch records stay intact.
Paseo's use-branch-switcher.ts compares the selected identity; the native wire
also carries explicit remote identity and must not collapse it into text.

Validation: the shared frozen native gate passed fmt, all-target clippy with
warnings denied, and 541 runtime tests plus two documentation tests. The
independent Info control uses this same source gate; no unchanged gate rerun.
No Git screen, node write or live input-method proof is claimed here.

## Shared Info toast for a known unavailable action

[Decision Log]
- Purpose: Paseo's git/use-actions.tsx38–58 explains unavailableMessage with a
  default-chrome toast, foreground Info16 and duration3200, before returning
  without calling the action handler.
- Constraint: existing Plain/Error renderer callers and Ok4s/Error-persistent
  semantics remain unchanged; no second card renderer or clock I/O in view.
- Alternatives: area paint, a successful/error toast for advice, or a timer that
  restarts the full lifetime on every hover-out.
- Choice: additive ToastLook/ToastTone Info, shared toast_with entry and typed
  optional hover callback, injected monotonic time and keyed running phases.
- Why: toast-host.tsx has no close control for this card, pauses the remainder
  on hover, and rejects dismissals for a card that has been replaced.
- Cost: old timer tasks may wake and be rejected; only the new Info replaces
  older Info. Paseo replaces every previous kind; retaining the legacy stack
  and persistent errors is an explicit compatibility departure, not parity.

The app/areas/toasts.rs bridge is the necessary shared caller seam: exhaustive
Info look, monotonic start/wake/hover, and optional typed hover handlers. The
same renderer keeps legacy chrome and dismiss controls; Info has no close.
Tests cover exact3200 vs legacy4s/no-expiry, the final millisecond, early wake,
remaining-time pause/resume, duplicate start, stale id/timer/hover-out, replacement
while hovered, dismissal while paused, and a persistent legacy error surviving.
Actual wall-clock/hover captures, legacy pixels and a kept-open Git menu with
zero node writes are still required; model tests do not claim that UI evidence.

The first integration attempt exposed a source-installation hazard: preserved
modification times let Cargo retain an older cached View library, so that
attempt did not prove the changed View code. Prepared source is now installed
with current timestamps; a reported gate must show the changed crates actually
checked/recompiled. No cargo clean is used. The subsequent real compilation
caught the clock's required Clone, unused return values and a scheduler that
needed no self parameter; explicit validation
and whole-result test assertions replaced those omissions. No failing attempt
is represented as a passing gate.

Validation: whole native fmt and all-target clippy with warnings denied passed;
541 runtime tests plus two documentation tests passed. Logs explicitly show
View, Kit, Link and App compilation, and all 360 frozen source/manifest/font
fingerprints still match after the gate. The branch-identity correction is an
independent preceding commit from this same gate. Native wall-clock/hover,
legacy pixel comparisons and actual Git no-write first use remain pending.

## Integrating the reviewed code panel before Git first use

The workspace owner's 212ed779 imports its tree context actions, directory-first
name shrinking, tree-row extraction, machine-book follow cancellation and Hub
unsubscribe path. This merge preserves its Files open_file entry and existing
code_git_changed refresh contract. No Git area registration, F-side getter,
file opener wrapper or header action slot is added by this integration; those
must land together with their own first callers and tests. F explicitly agreed
to a single last-child file_action_slot call after the header's counts, with
identical Some/None footprint from the kit's shared trigger geometry. Neither
that future call nor its runtime proof is claimed by this merge.

Validation: the merged native tree passed fmt and all-target clippy with
warnings denied; 545 runtime tests plus two documentation tests passed.
The changed View, Link and App crates actually recompiled; all 367 frozen
source/manifest/configuration/font fingerprints match. Only the reviewed F
line is imported: no root/node implementation was authored, no new root test
run or Git first-use/runtime approval is claimed. The separate README heading
correction and resume record follow as documentation only, without a Rust
gate rerun.

## Credit-stop continuation record

The closing F integration is committed as
`15adf0c3d986df9175b4bda79cddc7ce7d1bf300`, after its whole native gate.
`backend-m12-g1-resume.md` records committed/reviewed scope, the unregistered
prototypes and their source-only archive, exact missing APIs/callers/evidence,
and what F/G2/the lead are waiting on. The README's empty heading and altered
Rules heading are corrected in this documentation-only follow-up, as V1 asked;
no Rust gate is repeated for it. Git screen (b) and pull requests remain
unfinished and unapproved; neither is registered by this closing slice.

At 20:55:07 UTC all own fake nodes/harnesses/probes and the verified virtual
display were stopped by PID; no build remained running. The isolated failed-read
index was restored to 0644. Execution used UID/GID 1000, with no production owner
or mode change. No push, release or preview-node operation was performed.

## G1b, slice 1: the git toolbar on screen (2026-11-14)

Track G1 was taken over by G1b. The previous worker's unregistered
prototype (`app/src/areas/git/`, `link/src/git.rs`, `scripts/git-fleet.sh`)
did not compile against the current base and depended on kit menu
controls not yet on the lead's line; it was set aside outside the
repository, read, and not reused. The committed view models were reused.

### What is drawn, and from which reference file

| Part | Reference followed | Where |
|---|---|---|
| The repository row: branch leading, actions trailing | Paseo `git/diff-pane.tsx` `ChangesRepositoryToolbar` | `app/src/areas/git/head.rs` |
| Branch label trigger, its tooltip, the list | Paseo `components/branch-switcher.tsx`, `hooks/use-branch-switcher.ts`, `components/ui/toolbar-label-trigger.tsx` | `head.rs`, `branch.rs` |
| Split button: primary half, caret, menu | Paseo `git/actions-split-button.tsx`, `git/action-icons.tsx`, `git/use-actions.tsx` (`useGitActionRunner`) | `head.rs`, `menu.rs` |
| Commit form | GOAT `components/GitActionsControl.tsx` 1898–2072 | `commit.rs` |
| New branch / rename form | the page's `features/workspace/branch-dialog.tsx` (Paseo has none) | `branch.rs` |
| Saved entries, drop confirmation | the page's `stash-dialog.tsx`; Paseo's confirm dialog shape for the drop | `stashes.rs` |

### Rules added to `view`, with tests

- `git::step` (`step.rs`, `step_tests.rs`): what choosing an action does,
  decided again at the moment of choice. An unavailable action is
  explained in an Info toast and nothing is sent (Paseo's
  `useGitActionRunner`); forms open; Push sets the upstream when the
  branch has none (the page's `git-bar.tsx`); stash saves with or without
  new files.
- `git::primary_control`: the primary half's action (none when nothing is
  needed: the half is the git glyph and opens the menu, Paseo's
  menu-only trigger), whether it is the write running (spinner), and
  whether the panel has room for its words.
- `git::committed_state`: the one change a commit form's row names (the
  index side for staged-only commits, else the working tree's).
- `git::switch_stash` (`offers_stash`, `stash_for`, `saved`): Paseo's
  Stash & Switch, offered only for a branch switch refused for local
  changes; the switch is sent only after the save succeeded.
- `Reading::outstanding`: the key of the read in flight, for answers that
  echo a root rather than an app key.
- Git call failures gained the lead's `Problem::Unsent` ("Could not send."
  / "Try again."), with a test per language.

### How answers are matched

The node's workspace answers echo the root (reads) or the exact write; they
carry no app key. A read is taken when its root is the repository followed
and a read of it is out; a write when it equals the one write in flight.
A keyed link channel was rejected: it adds a command, an event and arms in
the shell's router and the unsent settler (shared files) for a protection
the root and the single write slot already give. The one cost: a read of
the same repository from before a leave-and-return is taken for the newer
one, and is read again five seconds later.

### Seams in track F's files (each one line or one getter)

- `workspace/code/changes.rs` `git_head`: its body calls
  `crate::areas::git::slot` (the slot F reserved).
- `workspace/code/mod.rs`: `Shell::code_git_context` (machine and root
  while the panel is out on Changes) and `Shell::code_git_width` (the
  panel's width, for the label's room), both read only, as agreed.
- `workspace/mod.rs` `workspace_answer`: `Status`, `Stashes` and `Git`
  answers go to the git area; `Branches` goes to both (F's start card
  reads branches too).
- `areas/mod.rs`: the area's `mod` line and field.

### Differences from the references, each with its reason

- **Menu rows have no leading glyph, muted look, spinner or success
  check** yet: G2's kit menu controls are on the lead's merge list. The
  behaviour is Paseo's already (an unavailable row is chosen and explains
  itself; a running row says so in its words; every row waits while a
  write is out). Swapped when the lead's hash arrives.
- **Failures stay beside their control** (under the toolbar, or inside the
  form that asked) instead of Paseo's error toast: the app's rule, and it
  keeps git's own output under the explanation.
- **A detached head still shows the trigger** ("Detached at abc1234") and
  opens the list; Paseo hides its switcher without a branch name.
- **A branch name is one line, cut in its middle** in the header, the
  commit form, the switcher's rows and a saved entry (`app/.../git/name.rs`,
  the kit's `fonts::elide_middle` to the width a probe measured), the whole
  name in a tooltip where the place has one (header, commit form). Paseo
  cuts at the end (`numberOfLines={1}` in `toolbar-label-trigger.tsx` and
  `combobox.tsx`), which hides what tells `feature/…-레이아웃` from
  `origin/feature/…-레이아웃`. The header and the switcher keep the kit
  controls' interface face; the commit form and saved entries use the code
  face. Switcher rows have no tooltip (the kit's combobox row has none).
- **A saved entry's subject is split** (`git::stash_subject`, git's fixed
  `On <branch>: ` and `WIP on <branch>: ` prefixes; a ref name cannot hold
  `:`): the message wraps as words, the branch is its own one-line row.
- **Commit form**: the node commits by scope, so the scope is a select
  where GOAT has per-file checkboxes; rows name the change in one word
  where GOAT shows line counts (the status carries none); no "Commit on
  new branch" (no single node act; New branch… then Commit); the message
  is required.
- **New branch, rename, stash and the saved-entry list** are the page's,
  since the node offers them and Paseo has no screen for them.
- **Stash & Switch** is Paseo's (`hooks/use-branch-switcher.ts`): a switch
  refused because it would overwrite local changes asks "Uncommitted
  changes", and on yes saves the tracked changes, then switches
  (`view::workspace::git::switch_stash`). Paseo recognises the case from
  git's message text; the node types it (`GitRefusal::WouldOverwrite`).
  Paseo's later offer to restore an entry saved for the branch switched
  back to is not built: the node's saved entries carry no branch of their
  own (Paseo tags its own entries), so which entry belongs to a branch
  could only be guessed from its message.

### Resume (2026-11-14 04:40 UTC, stopped for the account window)

- Done: slice 1 fixes for the coordinator's findings (one-line branch names
  in four places, `main` as the fixtures' default with a commit-on-main
  state, saved-entry subjects split); lead merged at `3a1075a5`; gate
  fmt/clippy/test green; captures dark/en/1280 in `/tmp/work/m12-g1b/shots/slice1b`.
- Half-done: the capture matrix (light, Korean, 390 as a sheet, scales
  1.25/1.5/2 with `GPUI_X11_SCALE_FACTOR`; the default-branch warning on
  "git commit on main"; the fill-inside-rounded-shape and pointer-cursor
  checks over the dialogs and menus). Helpers: `/tmp/work/m12-g1b/scripts/`
  (`app.sh` own display with its own authority file, `drive.py`).
- Next: finish the matrix, report slice 1 to the coordinator, then slice 2
  (open a pull request, its facts on the board card and in the session).
