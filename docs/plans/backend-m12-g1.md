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
  chrome/placement value, with typed content selecting the code face only.
- Why: existing `on` callers stay unchanged; one tooltip surface serves both.
- Cost: touch still has no tooltip, as before. A narrow pointer capture checks
  code wrapping at 390; it does not claim that touch gains hover interaction.

The path’s inline middle ellipsis uses the pinned GPUI’s existing
`text_ellipsis_middle`, not a new truncation algorithm. GOAT’s RTL/bdi
`StartTruncatedPath` instead shortens the start. Keeping both the initial
project context and final filename/extension follows the coordinator’s
explicit middle-truncation instruction; the entire path remains in code.
