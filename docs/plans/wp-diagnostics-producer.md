# Protocol 24 diagnostics app consumer

## Purpose and current status

The coordinator resumed E4 after app package base 9 (`db42d9c5`), carrying
node `9c150c6e` and the assigned protocol-24 diagnostics call. This supersedes
the producer-unavailable boundary in the earlier bounded app package. The
previous V4 row173 approval, whole1302/0/2, corrected tests1304/0/2 and captures
are historical evidence only; they do not validate this follow-up.

Integration is in progress. The shared selected-connection adapter is reserved
behind the lead's G4 slot and is not applied before that source lands green.
Only the view slice is independently checkable at this checkpoint. No live
full-report, build, capture, new gate or review success is claimed yet.

## Decision Log

- Purpose: obtain real allowlisted daemon/process diagnostics while preserving
  per-host partial outcomes, correlation, source bounds and safe copying.
- Existing assets: `NodeClient::diagnostics`, immutable `DiagnosticReport`,
  shared `ConnectionContext`, actor-owned connection epochs and the existing
  `SelectedConnection` before-poll/completion guard. Existing Settings dialog,
  view state machine, technical formatter and cached Fluent domains remain.
- Alternatives: call a freshly resolved alias, create another epoch allocator,
  duplicate the SDK deadline/identity wrapper, or merge a node worktree head.
- Choice: capture the exact shared connection context in each Ask and Answer;
  the actor selects that immutable client once. A minimal SDK adapter uses the
  existing guard without changing call/follow/retirement behavior. Basic info
  and the SDK diagnostic request run independently under one bounded host slot.
- Why: reconnecting as the same node must still retire old intent. The SDK
  already owns its ten-second transport deadline and report/welcome identity
  comparison; the app should not recreate either rule or expose raw errors.
- Consequences: old peers keep basic facts and explicit Unsupported diagnostics;
  denied, busy, timed-out, invalid-identity and partial reports remain distinct.
  Retiring a physical connection cancels both selected reads and immediately
  removes old copyable data. Existing ticket and current-payload copy checks
  still apply. Original weak DeliveryGuard receipts are revalidated after UI
  queueing and immediately before copying, without retaining a client or transport.
  Duplicate answers cannot grow the bounded receipts beyond one per host. No root/node/wire implementation is modified by this package.

## Data flow and data policy

The app pins each Machine's `ConnectionContext` before scheduling. Ask and
Answer carry that context plus the existing run ticket. The link uses
`selected_connection` and four nonqueued host permits. `node.info` retains its
named ten-second bound; the independent SDK diagnostic call uses its canonical
`DIAGNOSTIC_REQUEST_TIMEOUT`. The wrapper preserves typed DiagnosticError and
selected-admission refusal separately, then the E4 edge maps them to closed
Problem values. No remote message or cause chain enters a report.

The view checks ticket, expected admission and response identity; shared
retirement and Fleet reconciliation fence same-node reconnects, alias removal,
rename and rebinding. No second resolver, connection cache or product epoch
allocator is added. The SDK and node remain responsible for Read authority,
nonforwardable admission, canonical report validation and bounded source work.

`DiagnosticReport.process` is rendered as this daemon's numeric self-process
metadata. Missing outer sources and missing nested memory remain partial,
including Unsupported on other operating systems. Canonical typed section JSON
retains timestamps, saturation and omission. The combined text still has the
128KiB UTF-8 cap and64-host bound. No source is normalized into healthy zero.

## Reference and deliberate differences

Paseo app-diagnostic-sheet185-194 disables refresh while collecting;
283-287 separately measures active latency;264-309 collects per-host diagnostics.
The survey area5 sections53-56 describes app/daemon/process/log diagnostics.
The existing refresh/action fence, report footer and copy policy are retained.

Active RTT remains deliberately omitted: the current public SDK has no active
latency/echo observation. Timing node.info would include scheduling/server work
and is not the reference's five-second active transport measurement. This is
independent of daemon API availability. Raw daemon/provider logs, arbitrary
process enumeration, environment and paths remain excluded by ADR0074. The
report's recent failed local node calls are a bounded code/time-only summary,
not a raw log tail. Desktop lifecycle/updater diagnostics remain unavailable;
provider diagnostics belongs to A9 and onboarding to its desktop owner.

## Verification planned

View tests cover numeric process formatting, nested memory partial state and
exact-context/ticket/epoch retirement. Framed link tests cover the real assigned
SDK request, independent basic failure, old-peer no-frame refusal, report-id
mismatch, structured Busy/Forbidden, retired admission before polling, and real
deadline/cancellation/permit reuse for both independent reads. Existing redaction,
copy, language and refresh assertions remain. New framing has whole EN/KO goldens.

Checks and builds use the remote gate only; local static checks do not invoke
Cargo metadata. One final native gate and targeted end-to-end captures close
this follow-up after the shared adapter and all checks are ready. The new
producer runtime evidence must use matching protocol-24-capable binaries;
older shared root binaries cannot prove a successful diagnostic call.

## Wording checkpoint

L reviewed the framing meaning/data shape on `c69df8e3` only. The requested
follow-up removes the wire version from user-facing copy and replaces generic
contexts with each explanation's actual purpose. It adds no variables or
privacy/security approval. Whole-sentence EN/KO assertions track these final
sentences. Compilation and UI verification remain pending.

View checkpoint: run `wp-onboarding-view-20261115T173141Z-c69df8e3` completed
fonts and the selected diagnostics tests,28 passed/0 failed/0 ignored on exact
`c69df8e3`. This does not cover the later wording/semantic correction or staged
app/link adapter. View all-target Clippy on `07ea44bd` then found three redundant
test qualifications; these are removed without weakening any assertion/lint.
Its failed run remains evidence, not a compile/test success. No full consumer
validation is claimed before the guarded adapter lands.

The typed Stopping admission refusal has a distinct recovery sentence. Mapping
it to the old build-unavailable problem would falsely claim the API is missing.
Only ErrorCode::Unavailable with the diagnostics detail selects Busy/Stopping;
a contradictory detail never overrides an authority failure code. All Problem
variants have whole EN/KO goldens; the framing acceptance before this small
addition remains limited to its previously reviewed FTL delta.

A replacement live admission reports ConnectionChanged rather than claiming
the reachable machine is offline. Missing/closed snapshots keep Offline; a
shared retirement invalidates the old admission without implying OS shutdown.
Semantic session-list refusal retains the admitted context and lets the node
apply diagnostics Read authority independently.
