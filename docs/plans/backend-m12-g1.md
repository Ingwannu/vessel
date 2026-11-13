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
