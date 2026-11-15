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
`link/sessions` resolves node identities and fetches missing sessions with deadlines.
`app/areas/sessions` draws history and metadata using the kit and typed locale API.
The shared session menu delegates lifecycle actions and navigation into this area.
The existing fleet supplies current titles, marks and statuses. No provider process
is started merely to list or inspect history.

## Verification

Narrow crate tests/clippy through the shared build slot per commit. One whole native
gate and fake-fleet capture matrix at package end; root gate for the node package.
No live preview, real agent invocation, push or production mutation is authorized.

## Current implementation limits

The first history implementation searches case-insensitive metadata terms locally
and pages the already fetched fleet fifty rows at a time. Paseo searches history on
the host with typo tolerance and server cursors. This is a deliberate initial
compatibility path because the existing node directory lists complete local rows;
server-side history querying remains a later implementation step, not claimed parity.
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
and OS deep-link delivery remain their respective packages. No node edits exist.
Next: tests, typed link boundary, history UI, then integration and package evidence.
