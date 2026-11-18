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

Superseded by TRACKER 802 (README, "Toasts are Paseo's"): every toast now
takes the keyed clock below and replaces the one up; the stack, the close
control and the persistent error are gone.

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
  face. The switcher is as wide as its trigger (at least 280, at most
  420; the screen's width on a phone), and a row cut in its middle
  carries its whole name as a code-face tooltip (`ComboRow::tooltip`,
  a control-only kit commit).
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

### Motion and idle cost (the owner's word 39)

- Transitions: the issue under the git row fades in over
  `motion::STANDARD` on `motion::CURVE`. The menu, the switcher, the
  dialogs and the tooltips are the kit's, and take track S3's kit
  transitions.
- Idle CPU, release build, 1280x800, software renderer under Xvfb, the
  pointer resting off every control, 60 s: the session with the panel
  closed 0.58 % of a core; with the git panel open 9.68 %, as 0.45 s of a
  core every five seconds and nothing between. The five-second status
  refresh redrew the whole window twice per refresh (when the read
  started and when it answered) even when nothing changed. Now a
  background read asks for no frame, and an answer equal to what is shown
  asks for none either: `Shell::git_status_read` says whether the drawing
  changed, `Shell::workspace_answer` passes that on, and the shell's
  `Event::Workspace` arm skips its frame (one arm in the shell, the
  router's return value in track F's file). Measured again the same way:
  0.57 % with the panel open, the closed panel's cost. A file made
  outside the app still reaches the git row within one refresh (its
  primary action turns to Commit). Track F's Changes list re-reads only
  after the app's own writes (`Shell::code_git_changed`), as before; the
  git area now knows when a refresh found a change, so one call there
  would make the list follow outside edits too (offered to F).

### Capture matrix (fake fleet, `scripts/git-fixtures.sh`)

1280 in both themes and both languages at scale 1, dark at 1.25, 1.5
and 2; the phone at 390 (forms as sheets); the small windows 1024x700 and
800x600 reached through the Board. Found and fixed through it: the
default-branch warning missing when `origin/HEAD` names the default
(the node marks `origin/main`), a branch name cut twice on its first
frame, and the commit form's 176-point file frame pushing the message
below an 800x600 window.

## G1b, slice 2: a session's pull request (2026-11-14)

### What the node gives, and what it does not

- The summary rides on the session (`SessionInfo.pull_request`: access,
  the last summary read, whether feedback goes to the session), kept fresh
  by the node's watcher, for live sessions with a project or a worktree.
  The app's cards carry it (`CardDetail.pull_request`, one field and one
  line in track S's `view/src/board/card.rs`).
- `pr.get` gives it whole (checks with counts and at most twenty failing
  runs, reviews' standing, merge methods); `pr.open` opens one from the
  session's branch (the node pushes it first for a person, never for an
  agent). Both travel on the workspace pair (`Ask::Pull`, `Answer::Pull`,
  `link/src/workspace/pulls.rs`), as `session.get` does: no new variant
  in the lead's `Command` or `Event`.
- `pr.list` covers watched (live) sessions only, so no project-wide pull
  request page is drawn from it.

### What is drawn, and from which reference file

| Part | Reference followed | Where |
|---|---|---|
| Create PR / View PR in the git menu, after the remote actions, and why not | Paseo `git/policy.ts` (`buildPrAction`, `getCreatePrUnavailableMessage`, `secondaryIds`), `git/use-actions.tsx` (`handlePrAction`) | `view/src/workspace/pull/{entry,menu}.rs`, `app/src/areas/git/menu.rs` |
| The link at the head of the Changes tab | Paseo `git/diff-pane.tsx` `ChangesPullRequestLink`, `git/pull-request-state-icon.tsx` | `app/src/areas/pulls/link.rs` |
| The facts | Paseo `git/pull-request-panel/pane.tsx`, `checks-summary.ts`, the `checksOverview` words | `view/src/workspace/pull/facts.rs`, `app/src/areas/pulls/facts.rs` |
| The Create pull request form | `gh pr create` (title, body, base, draft) and its `--fill` defaults; GOAT's form layout as the commit form | `view/src/workspace/pull/{open,bases}.rs`, `app/src/areas/pulls/form.rs` |
| Words | Paseo `i18n/resources/{en,ko}.ts` (`workspace.git.actions`, `workspace.git.pr`, `panels.pullRequest`) | `view/locales/{en,ko}/pulls.ftl` |

### Differences from the references, each with its reason

- **A form where Paseo sends none**: Paseo's daemon writes the title and
  description with a model; this node takes the person's title
  (`PullTitle`) and never prompts an agent. Its defaults are `gh`'s:
  one new commit gives its subject; several give the branch name with
  `-` and `_` as spaces and their subjects, oldest first, as a list. The
  base defaults to the one the worktree was made from (Paseo's stored
  base ref), else the node's default branch.
- **The facts are a dialog** opened from the link, where Paseo's are a tab
  beside Changes: a tab is track F's panel and tab bar; a dialog needs
  neither. View PR (the menu) opens the page, as Paseo's does.
- **Checks are counts plus the failing runs**, and Activity is the
  reviews' pills (approved, changes requested, unresolved threads), the
  verdict and mergeability: the node sends no passed or skipped runs by
  name and no timeline. "Add to chat" comes with the feedback slice.
- **Merged is drawn in the accent**: Paseo's purple is not in this
  palette.
- **The node's finer reasons** (signed out, refused, rate-limited,
  unreachable, at its watch limit, session ended) each have their own
  sentence in Paseo's voice, where Paseo has one "isn't connected".

## G1b, slice 3: merging a session's pull request, its feedback, and the fake forge (2026-11-15)

### What is drawn, and from which reference file

| Part | Reference followed | Where |
|---|---|---|
| Merge PR (squash), (merge), (rebase) after View PR, shown only when a merge would be taken | Paseo `git/policy.ts` `canMergePr` (560-591), `shouldShowPullRequestAction` (753-770), `buildDirectPullRequestMergeAction` (444-462) | `view/src/workspace/pull/{merge,menu}.rs`, `app/src/areas/pulls/merge.rs`, `app/src/areas/git/menu.rs` |
| Why a merge was refused | Paseo `getMergePrUnavailableMessage` (712-747) and its `unavailable.mergePr*` words; the node's closed `MergeRefusal` | `view/src/workspace/pull/merge_words.rs` |
| The feedback switch | The node's `pr.feedback` (`SetFeedback`); a row in the facts dialog | `app/src/areas/pulls/feedback.rs` |
| The facts header | Paseo `pull-request-panel/pane.tsx` 524-552 and its styles 1216-1224 (the title at the body size and weight, the number muted in the same run) | `app/src/areas/pulls/facts.rs` |

### Differences from the references, each with its reason

- **The rule reads what the node sends**: Paseo's `mergeCapability`
  (direct merge ready, auto-merge, merge queue) is the node's
  `mergeable` (Unstable counts as ready: only checks that are not
  required fail) and its refusals (`MergeQueue`, `NotMergeable`); its
  `aheadCount > 0` applies only where the node knows the base (a
  worktree made from a branch).
- **The pull request is pinned**: `pr.merge` carries the number and the
  head seen; a push since is refused as `HeadMoved`. Paseo merges on
  one choice without a confirmation, and so does this.
- **A merge row is a consent**: `merge_methods` takes the pull request
  and its summary apart whole (README's rule for types a person
  accepts), so a field a newer node adds stops the build until the rule
  reads it.
- **Auto-merge is not offered**: the node has no auto-merge call.

### Found on the fake forge fleet (ADR 0060, the node line's `vessel-forge-fixture`)

- The form showed, and would have sent, the base `origin/main`. The node
  measures a project against its default as it marks it, and sends
  `pr.open`'s base to the forge as written; it strips `origin/` only for
  its own default. Fixed in `view` (`start_base`): absent (the node's
  default, `gh pr create` without `--base`) for the project's default
  in either spelling, and a remote-tracking branch without its remote's
  name.
- The facts title put its number on a line of its own when it wrapped.
  It now follows Paseo's pane.
- At 800x600 the code panel sits over the conversation, above a scrim
  that closes it on a press. The panel did not occlude, so a press on the
  pull request link reached the scrim, the panel closed, and the facts
  never opened. Fixed with one line (`.occlude()`) in the panel's
  `over()` (`app/src/areas/workspace/code/frame.rs`, the lead's 0.0.0.0.17
  change), as the kit menu, the palette and search already do.

### Captures

At 1280x800 in both themes and both languages:
- The empty case: the menu, the form, the base list, and the refusals
  for a blank title and a title too long.
- The ready case: the link, the menu, the facts, and the feedback switch
  turned on and off.
- The refused merge.
- The failing checks.
- No access.

In dark en and light ko:
- Changes requested, rate-limited, draft (no merge rows), conflicting
  (no merge rows), pending and unknown.
- The read-only person, whose merge and open the node answers Forbidden.

Also captured:
- Dark en and light ko at scales 1.25, 1.5 and 2: the empty case's menu
  and form, and the ready case's link, menu and facts. The link's glyph
  is enlarged at the four scales.
- 1024x700, 800x600 and 390x844 in dark en and light ko: the form and
  its blank refusal, the link, the menu, the ready and failing facts, and
  the refused merge.
- The 800x600 facts shots were taken before the occlusion seam. They show
  the bug (the panel closes and no dialog appears), not the fix.

Not yet captured (the session was paused):
1. 800x600 again with an app built at or after the seam: the ready and
   failing facts.
2. Last, because they change the fleet's forge: `pr.open` on the empty
   case and a squash merge of #301 on the ready case. Both are at 1280x800,
   in dark en, followed by the board's card lines.

Not drawn: `OpenOutcome::Exists`. It needs a pull request opened between
the menu's read and the form's send, which the fixture cannot stage.

### Resume

- The branch is the lead's `f54b3633` with this track's work as one
  commit per area (view: start base; view: merge and feedback; link;
  native; docs), then the occlusion seam, then this note. The pre-split
  history is kept as `backend/m12-app-git-presplit`.
- Gates: the native gate runs on the second build machine
  (`vessel-remote-gate ... --only native`, label `gitpr`). Its run id and
  totals for the committed head are in the track's notes outside the
  repository, and in the report to the coordinator.
- Next: the two capture items above, then hand the slices to the lead
  with the capture folder. The project-wide pull request page waits on
  ADR 0061.
- Seen, not mine: from the sidebar, "All projects (14)" opens
  build-server's Projects page, which says "No projects yet" (the
  projects are on laptop).

### Resume (2026-11-14 15:50 UTC, paused by the coordinator for the account windows)

- Slice 3 (merge and feedback) is a WIP commit on this branch, after the
  hand-off head `0036db7c`: `view::workspace::pull::{merge, merge_words}`
  with tests, the merge rows in `pull::menu`, the words in `pulls.ftl`,
  `PullAsk::Merge`/`Feedback` and their answers in the link,
  `app/src/areas/pulls/{merge,feedback}.rs`, the menu's merge rows and
  the facts' feedback switch. Clippy is clean on it (gate p4); its tests
  were not run (the gate was stopped at the pause). Resume: run the whole
  gate, fix what it finds, then split the WIP commit by responsibility
  as slice 2 was.
- On waking (the coordinator, once track N3's fake forge is on the node
  line): first merge the lead's line at or after `ae8c1d38` (0.0.0.0.16,
  which holds `0036db7c`), then capture the Create PR form, then the
  facts, the link, the card line, and slice 3's rows and switch.
- What the lead changed while merging `0036db7c`:
  - The picker keeps both `Leading` forms (Mark and SmallGlyph). The
    combobox draws the row tooltips, its row arguments are bundled in
    one `Listing`, and combo rows moved to `kit/src/picker/rows.rs`.
  - `code_git_width` is the width the frame draws (`PANEL.width`, for
    word 29's edges), not the field.
  - The screens' literal sizes in git/head, menu, stashes and commit, and
    in pulls/facts, form and card, became named constants and spacing
    steps.
  - The form summary's colour is `theme::SUMMARY_LIGHT`.
  - The WIP commit `76fedfb7` will conflict where it touches those
    files: resolve toward the lead's names.

### Resume (2026-11-14 15:00 UTC)

- Done: slice 1 with its matrix fixes and the idle fix; slice 2's rules,
  link calls and screens; gates green (777 tests).
- Waiting on others: track N3's fake forge for the fleet (the facts, the
  link, the card line and the form's refusals are drawn against Paseo's
  pane but captured only up to "Create PR isn't available" until a node
  can read a forge); track S3's kit transitions for the menu, the
  switcher, the dialogs and the tooltips; G2's kit menu controls.
- Next: the merge slice and the feedback switch. Design, from Paseo's
  `git/policy.ts` (`canMergePr`, `buildDirectPullRequestMergeAction`) and
  the node's contract (`forge/calls.rs`):
  - Paseo merges on one choice, with no confirmation: "Merge PR
    (squash)", "(merge)", "(rebase)" (ko "PR 병합(스쿼시)" …), pending
    "Merging PR...", done "PR merged". The rows show only when the pull
    request is open, not a draft, not conflicting, and the branch is
    clean and in step with its remote (Paseo's `aheadOfOrigin` and
    `behindOfOrigin` zero, `hasUncommittedChanges` false); each method
    only when the repository allows it.
  - The allowed methods are in `pr.get`'s whole pull request
    (`merge_methods`), not in the card's summary: the git area reads it
    when its session has one and again after each refresh that changed
    the summary's `updated_at`.
  - `pr.merge` pins the number and the head the person looked at
    (`summary.head_commit`); a push since is `MergeRefusal::HeadMoved`.
    Each refusal gets Paseo's sentence where it has one (`mergePrDraft`,
    `mergePrClosed`, `mergePrConflicts`, `mergePrQueue`,
    `mergePrNotReady`), else its own in Paseo's voice; the forge's own
    words go under it as for `pr.open`.
  - Feedback (`pr.feedback`, `SetFeedback`): a switch in the facts dialog
    ("Send failing checks and reviews to the session"), its state from
    `SessionPullRequest.feedback`.
  - Captures wait for track N3's fake forge, as slice 2's facts do.
- Capture helpers (outside the repository): `app.sh` (own display, own
  authority file), `drive.py`, `matrix*.sh`, `idle-cpu.sh`. The menu's
  rows below Fetch move down by one group when Create PR is offered:
  scripted clicks below Fetch must use the row positions of the session
  driven (a click meant for "Stashes…" stashed the fixture once; it was
  restored from that entry).

### PR closeout: review wording and the small facts window (2026-11-15)

- Existing assets and decision: modify the existing `ReviewPills` projection,
  shared facts dialog and Fluent catalogue; no new control, dependency or wire
  call. The standard library and the current typed projection suffice.
- Reference: Paseo `git/pull-request-panel/pane.tsx:259–267, 575–604`
  counts approvals and requested changes once in Activity's summary. Vessel
  receives review standing rather than that timeline. Keep its existing
  documented summary, but suppress a verdict only when the matching standing
  pill already says it. A differing verdict (including review required) stays;
  hiding every verdict would discard a fact the forge supplied.
- Tests cover all five verdicts against all four approval/change-request pill
  combinations, plus whole English/Korean labels and the form title.
- Translation review settled the Korean form title as `PR 생성`, matching the
  menu action. English retains the reference's contextual distinction between
  `Create PR` and `Create pull request`. Only the Korean title key changes.
- Resume point: the inherited source passed its native gate. This closeout's
  remote gate and fresh executable captures are pending. Retake ready/failing
  facts at 800×600 after the existing code-panel occlusion seam, then exercise
  Create PR and squash merge against the disposable file-seeded forge, and
  inspect the board. No real forge or agent is used.

#### A finding from the final squash-merge run

The real UI write completed: its toast said `PR merged`, the header's glyph
changed, the refreshed facts said `Merged`, and merge rows disappeared. However,
the fixture's summary retained `mergeable`, so Activity still said `Ready to
merge` beneath `Merged`. That is historical data, not current readiness. The
facts projection offers readiness only for a non-draft open request.
Draft, closed, merged and unknown requests omit it; their state remains in
the header. All six mergeability values are tested across those states and both
open/draft forms. Review pills and a distinct review verdict remain unchanged.
This follows Paseo's state header and Activity summary (the pane has no extra
readiness line); Vessel's additional readiness fact remains for open requests.
The previous closeout gate passed 1139 tests with none failed and one ignored;
this changed tree needs its own gate and merged-facts retake.

### PR closeout result and resume point (2026-11-15)

- Code: `175491aa` removes matching review-verdict repetition and unifies the
  Korean form title; `d230934a` removes historical readiness from non-open
  facts. Both extend existing view rules and the existing facts drawing.
  There is no node, wire, dependency or shared-control change. The only
  translation edit is the agreed Korean title key.
- Native gates: all six steps (fonts, fmt, quality, clippy with warnings
  denied, app build, tests) passed on each code commit. `175491aa`: 1139
  passed, none failed, one ignored. `d230934a`: 1140 passed, none failed,
  one ignored. Each changed crate actually recompiled. The narrow clippy
  checks passed too. Root source is unchanged; no root compile gate was run.
- The final capture set has 41 individually inspected images. It includes
  ready/failing facts at 800×600 in all four theme/language combinations,
  forms and facts at 390×844, 800×600 at scale 1.5, successful creation and
  squash merge at 1280×800, their board card lines, and merged facts at
  1280, 800 and 390 in dark English and light Korean. Scrolled failing
  facts demonstrate footer and feedback access. The manifest pins actual
  pixel dimensions, image hashes, source commits and fresh executable hashes.
  Thirty-three captures use `175491aa`; eight terminal/scrolled retakes use
  `d230934a`. One obsolete contradictory merged-facts capture is separate
  failure evidence, excluded from the final set.
- The adapted final-forge script ran once on a fresh disposable fleet. UI
  creation made request #311; UI squash merged #301. An independent
  `pr.show --refresh` confirmed both outcomes and local-only push targets.
  The header, menu and board updated. Fixture initialization checked the
  other seeded states and read-only refusals before these writes.
- The fixture completed writes before the first shot: those captures are
  named for their success toast, not claimed as pending-state evidence.
  Not verified: real forge accounts, browser opening, Windows, the
  `OpenOutcome::Exists` race, write-pending visuals, or closed/unknown
  pull-state UI (the latter states have pure rule coverage). This closeout
  does not implement the project-wide pull request page.
- The old WIP/resume entries above are historical. Current committed code
  and its gates supersede them; the inherited `54b8aaa1` gate passed 1137
  tests with none failed and one ignored. The original checkout was left
  untouched; finishing used a separate checkout and build target.
- All disposable nodes, the app and its authenticated virtual display were
  stopped by verified process identity. No production permissions changed.
  Artifact locations and run identifiers are in the private review report.
- Next: source and visual review, then native-lead integration for build
  0.0.0.0.19. No push or release was performed by this slice.


### Draft-readiness source review correction (2026-11-15)

The review returned one source finding: state, draft and mergeability are
independent wire fields. An open draft can report `Mergeable` or `Unstable`;
the previous test incorrectly required every draft to expose that reading.
`Merge::for_open` now returns no current readiness for any draft. The Draft
header, checks and review facts remain; it neither invents a Blocked response
nor changes merge permissions. Paseo `git/policy.ts:560–591` and the existing
`merge_methods` already refuse drafts. Explicit expected results for all six
non-draft open outcomes remain; terminal/unknown states are tested with both
draft flags for all six values. This is a wording projection correction,
not a node or transport change.

The previous 41 captures and their source assignments remain unchanged.
The reviewer checked each original image, dimension and hash, with no
additional visual return. A fresh independent Draft+Mergeable/Unstable seed
and changed-tree native gate are the remaining verification for this fix.
No old screenshot or old passing gate is claimed as proof of this correction.


#### Draft correction verification result

`6406211c` passed one changed-tree whole native gate: fonts, fmt, quality,
clippy, build and tests all exit zero; 1140 passed, none failed, one ignored.
The changed crates actually recompiled and the corrected regression ran.
The narrow clippy check and fresh local debug build also passed.

The copied complete fake-forge seed independently admits Open+Draft with
Mergeable (#301) and Unstable (#306), verified through `pr.show --refresh`.
Sixteen additional original captures were individually inspected: facts and
complete menus for both shapes at 800×600 and 390×844 in dark English and
light Korean. They retain Draft/check/review facts without current readiness
or merge rows. Their manifest pins source, executable and image hashes.
The old 41 images, their source assignments and their gate logs are intact.
They were neither redone nor presented as proof of the draft correction.

An initial title-query capture did not identify the intended PR and opened
a branch chooser; it is excluded diagnostic evidence, with cause undetermined.
The retake selected the session by its stable ID and verified the PR number.
A restart blocked by an owned stale display lock was also stopped safely;
cleanup followed dead-process and refused-socket checks. Neither diagnostic
was addressed by changing product code or weakening an assertion.

The disposable fleet, app and display have stopped. No node/wire/shared-control
or other-area source changed. The same real-forge/OS/Exists/pending/closed-state
UI limits remain. Bounded source, gate and inspected-visual review passed
for exact code `6406211c` and documentation `ea26773b`; the sole draft-readiness
return is closed. The review independently checked all 16 new originals,
their dimensions and hashes, both raw node replies and the six gate results.
All old 41 originals and their 33-at-`175491aa`/8-at-`d230934a` assignments
remain intact; they are not relabelled as observations of `6406211c`.

This is a bounded lead candidate, not approval of real forge accounts, OS or
browser behaviour, Exists/pending/closed-or-unknown UI, the older 167-image
set, the project-wide PR page, or a later combined tree. Nine diagnostic
images remain excluded and their initial mismatch has no proven cause.
The selected own code commits and documentation go to native integration;
they do not replace the lead's whole branch. No unchanged gate was rerun,
and no release or push was performed.
