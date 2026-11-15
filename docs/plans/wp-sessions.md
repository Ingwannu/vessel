# Session lifecycle in the native app

Status: implementation in progress.

## Reference and existing assets

Paseo 0.15.4: `packages/app/src/screens/sessions-screen.tsx:24-160`
keeps search separate from transcript search, filters hosts and reports partial-host failures.
`packages/app/src/subagents/select.ts:77-104` selects unarchived managed children
across hosts and sorts them oldest first. `subagents/close-tab-policy.ts:5-13`
archives roots on close but removes only layout for children.
`packages/protocol/src/agent-deep-link.ts:28-73` routes an explicit host and session;
credentials, query and fragment are rejected by the desktop link parser.
`packages/app/src/hooks/use-archive-agent.ts` owns archive cache rollback;
`subagents/use-detach-subagent.ts` performs explicit detach.

Existing assets: `view::board::Fleet::tree`, `Card::from_session`,
`link::session::Act`, board and conversation confirms, `Shell::open_session`,
mark/resume/delete calls and title folding are reused. The archive is a board dialog,
not a chronological sessions screen. The sidebar tree caller belongs to its existing
owner. Import/history discovery and transcript search retain their separate owners.

## Decision Log

- Purpose and intent: expose durable sessions, lifecycle operations and parent/child
  navigation consistently without coupling screens to network code.
- Existing constraints: app and node bases are converging; labels and reload are
  staged. A creator is immutable provenance, so detach must not rewrite its history.
- Alternatives: overload archive as history; introduce a second lifecycle command
  dispatcher; filter child identity by mutable title; external UI/data dependency.
- Choice: modify existing lifecycle actions and projections, add isolated `sessions`
  modules for chronological history, navigation and metadata. Reuse existing kit and
  standard collections; introduce no dependency. Workspace archive and restore build on the shared workspace identity contract.
  Managed child detach belongs to its separate subagent package.
- Why: existing confirmation, queue rejection and typed node errors remain the one
  path; pure selection rules can be checked without GPUI or real providers.
- Advantages, costs and effects: small registration seams; independent history UI
  and cross-host addresses; explicit compatibility gates while node features stage.

## Differences from Paseo

- Link scheme and entity names use Vessel node/session identifiers.
- Existing node lifecycle semantics (ADR 0063) govern archive/resume/reload; no app
  request bypasses an unsupported protocol.
- Provider-native subagent tracks require a separate node contract; this package
  concerns managed sessions with a durable creator relationship.

## Structure and data flow

`view/sessions` owns chronological filtering, paging and lifecycle policy.
`link/sessions` resolves node identities and fetches missing sessions with deadlines. A coalesced watch
snapshot retains one destination through reconnect under one deadline; newer
requests abort older metadata reads. An offline alias is refused because it
cannot safely identify a replacement host.
`app/areas/sessions` draws history and metadata using the kit and typed locale API.
The shared session menu delegates lifecycle actions and navigation into this area.
The existing fleet supplies current titles, marks and statuses. No provider process
is started merely to list or inspect history. Existing Tokio test utilities are
enabled only for deterministic virtual-time reconnect/timeout tests; no runtime
dependency is introduced.

## Verification

Narrow crate tests/clippy through the shared build slot per commit. One whole native
gate and fake-fleet capture matrix at package end; root gate for the node package.
No live preview, real agent invocation, push or production mutation is authorized.

## Current implementation limits

History searches title and branch metadata through the shared matcher with
opt-in per-token typo tolerance, then retains activity chronology and grows the
visible window fifty rows at a time. Default command/catalog matching is unchanged.
The canonical registry must still contribute workspace/project display names; host
labels, provider names, directory paths and transcripts are not surrogate fields.
Paseo performs this search on the host with server cursors. The current node lists
complete rows, so local paging is a compatibility path; bounded host history
querying remains unfinished and is not claimed as cursor parity.
The current host failures are explicit; offline hosts do not contribute invented rows.
Fresh conversation refuses before archiving when a separate new-chat draft already
exists or when a named profile cannot be carried by the current draft contract.
This preserves unsent work and launch identity until the shared tab/profile
contracts provide an exact destination. Archived sessions hide their composer
from the fleet snapshot before the session metadata request has completed.

## Resume point

Done: reference/asset survey; chronological history, host selector and date/page
models; correlated navigation and lifecycle shapes; initial narrow view check
passed 20 tests including six package regressions. Fast size/boundary/token checks
pass. Session identity, link stream forwarding and shell event dispatch were
mechanically extracted to avoid increasing shared file budgets.
In progress: native session confirmations and success-only close/fresh navigation;
framed fake-peer link tests and narrow clippy. The first link clippy exposed helper
test-cfg lint handling and a dispatcher function at its length boundary; repairs
retain the original results and do not weaken the lint configuration.
Ownership: final single-owner table assigns A2 here; subagents/detach, labels
and OS deep-link delivery remain their respective packages. The separate node
worktree stages ownership-fenced workspace archive/recovery behind a typed fake;
its reviewed registry/store dependency is not yet consumer-ready.
Next: narrow checks, current-parent relation and pane close integration, registry
adapter plus pending workspace calls, then package capture/gate evidence.


## History matcher decision

Existing `view::matcher` already implements the reference exact/subsequence tiers
and UTF-16 ranking with UTF-8 marks. Modify it with an additive opt-in constructor
and isolated bounded-edit module instead of copying the matcher or adding a
third-party fuzzy library. Budgets follow `protocol/src/search/text-match.ts`: no
typos through three UTF-16 units, adjacent swaps only at four, one edit through
seven, two above seven. ASCII words and leading slices are the typo candidates;
whole-word highlights identify a typo match. Existing picker defaults and tests
remain the compatibility contract.


## Authenticated passive access

The lookup result retains a `sessions::Binding` (validated host name plus exact
node/session address). After W10 reserved `conversation_opened` for this seam,
its metadata/conversation reads and the log follow use `SessionAccess` instead
of alias-only commands. The link actor validates the live `NodeClient` identity
and captures it synchronously before dispatch; alias replacement refuses without
contacting the replacement. Lifecycle requests retain the same optional expected
node, preventing an archive/delete on an alias that changed after lookup. The shared `Machine.node` welcome identity now hydrates ordinary saved-host
opens too; already-bound deep links retain their original expected identity.
Legacy records without a hydrated identity keep their existing passive path. Queue/web refusals settle the exact pending read/stream.

Evidence so far: the standalone staged node workflow's first 11 tests pass
(in-memory ports only), and native view's full 870 tests pass. Node concurrent
archive exclusion and native scoped-read/follow changes are newer and still
require their exact-head checks. No full gate, capture or runtime parity claim.


Composer `/exit` and `/clear` execute immediately, as
`client-slash-commands/index.ts:17-36` and `panels/agent-panel.tsx:1579-1610`
specify; they do not reuse the menu's Archive/Delete confirmation dialog.
Vessel deliberately retains the visible session until authoritative archive
acknowledgement instead of optimistically closing/retargeting before the RPC.
Busy/refused/uncertain outcomes keep the session available with an inline error.
