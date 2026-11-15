# App diagnostics consumer

## Scope and reference

The current package owns the existing Settings > Diagnostics page, not onboarding.
The first-run walkthrough remains ADR 0071 and its desktop owner.

Paseo source observations (package-relative paths, inspected 2026-11-15):
- `app/src/components/app-diagnostic-sheet.tsx:37-152`: pending/running/done/failed
  rows for client, desktop and each saved host; a run identity supersedes results
  after close/refresh. Copy is enabled only for a completed report.
- `app/src/diagnostics/app-diagnostic-report.ts:108-142`: redaction covers saved
  connection values and credential assignments before copy.
- `app/src/components/app-diagnostic-sheet.tsx:230-309`: disconnected hosts and
  unsupported diagnostic collection stay explicit rather than becoming healthy.
- `app/src/components/provider-diagnostic-sheet.tsx`: provider-specific report
  sheet; model catalogue/settings belong to provider management, not diagnostics.
- The feature survey sections 53-56 also describe process facts and desktop
  daemon log viewing. These require a real producer; session/terminal/git logs
  are not daemon logs.

## Decision Log

- Purpose: preserve independent host outcomes, bound the combined copy/export,
  and keep late results and confidential node fields out of a support report.
- Existing assets: extend `view::diagnostics`, the existing Settings diagnostic
  sheet, device MachineKey/Ticket, existing paired NodeClient connections and
  Fluent catalogs. ADR 0074 owns the node report and its privacy boundary.
- Alternatives: add a second page; export NodeInfo or agent readiness wholesale;
  run provider discovery as an automatic diagnostic; read arbitrary log files.
- Choice: modify the existing page and add a pure collection state machine,
  a bounded typed link command/event, and a source allowlist. No dependency,
  shell, agent, environment scan, arbitrary file read or new RPC is introduced.
- Why: node info contains names, routes and a public key; agent status can spawn
  discovery even with refresh=false. Neither is a safe diagnostic substitute.
- Consequences: the current node-info read may report identity/protocol/lifecycle
  only. Full daemon/provider/process/log producers remain explicitly unavailable
  until their reviewed contracts exist. Test fakes are cfg(test) only.

## Structure and data flow

`app/areas/settings/diagnostics` draws the existing kit dialog and dispatches
`link::diagnostics::Ask`. The link uses the selected existing connection, echoes
its admitted node id and request ticket, and projects a safe snapshot.
`view::diagnostics::collection` owns run/row transitions and bounds; rendering
uses Fluent framing while the technical report keeps English labels. A close
invalidates the active run; refresh reserves a new ticket. A changed/removed
book entry invalidates its pending row. Node report identity must match the
admitted connection identity. Missing sources never become zero values.

## Differences and producer boundary

ADR 0074 deliberately replaces raw daemon logs with scoped typed failed-call
observations, and excludes paths/environment/agent output. It has no public
node.diagnostics request at this base. Provider diagnostics, OS process lists
and daemon tails likewise have no current callable safe producer. Screens
state this; they never display a fixture result as a live successful report.
Local app renderer and preference/log-location facts stay available, with home
redaction and a bounded final output. Every unsupported section is named.

## Validation and resume point

Done: pure run/host state, bounded safe text, per-host progress and the existing
sheet extended, explicit unavailable daemon/provider/process/log source rows,
node.info projection and framed in-memory peer tests; EN/KO Fluent framing.
Narrow checks pending. Shared registration-only additions exceed the old shell
and link ratchets; waiting for the announced package base/shared extraction.
No size record is raised. Native lead has the exact registration footprint.
Next: compile narrow checks, consume the announced base and L framing, then
capture once and run one full native gate. The whole node diagnostic producer
remains unavailable; do not claim an integrated wire success.

### Narrow compilation record

The first narrow check found incorrect test enum names (`NodeInfo`, not
`GetNode`) and an unnecessary Result helper. The next found the kit calls
for `Type::Meta`, not Caption, and an unchecked test read length. These
were corrected without changing any existing test expectation. Typed source
fixtures now also reject a mismatched admitted node id. Compilation of the
corrected package is pending; no full gate or capture has run.

Collection now schedules four hosts at once instead of making every later
host fail busy. A failed refresh supersedes its predecessor even before a
replacement report is available. A removed selected host remains explicitly
offline. A source cap or the combined text cap refuses the whole result; it
never truncates a supposedly complete support report. New Korean framing was
reviewed against the existing machine terminology and complete recovery text.

The third narrow run compiled the changed view and link crates and found two
formatting-only app closure lints, now corrected. The exact current compile
check and diagnostic tests still need a green result. Unchanged offline rows
now keep their existing report allocation instead of rebuilding it per frame.

### Announced base integration

Merged the announced package base `8d5513e4`. Conflicts were limited to
`app/src/shell.rs` and `link/src/lib.rs`: the announced head won both files,
then this package's registration lines were reapplied. Command/Event variants
now register in `link/src/command.rs` and `event.rs`; dispatch in `dispatch.rs`;
app events in `shell/events.rs`. No base behaviour or expectation was changed.
The new unassigned-protocol client error maps to explicit unavailability.
Native format, size, boundary and token checks pass after the merge. The prior
package compile check passed on `6f6dac4f`; the merged code still needs its
updated narrow check and diagnostics tests. Shared file budget blocker resolved.

### Approved typed daemon report consumer

The announced base makes ADR 0074's immutable DiagnosticReport root available.
`DaemonSnapshot` now retains that root independently of the basic node-info
read. A partial typed report preserves successful sources and observation
times; unavailable sections stay explicit. Both the basic result and report
identity are checked against the admitted node. Combined report bounds apply
after every host. The product link still returns Unavailable for the unregistered
full report; only cfg(test) fixtures populate typed report values.

The technical renderer uses English section labels and canonical JSON for
nested allowlisted DTO rows, rather than flattening each scalar into a separate
English label. This deliberate formatting difference preserves saturation,
omission and cached/stale/unknown distinctions without a second wire schema.
Monotonic uptime, normalized version, closed OS/architecture and redacted home
are rendered directly. Recent failures are explicitly scoped local node-call
observations, never described as a daemon log or absence of errors. Extended
process metrics, provider diagnostics and raw daemon logs remain unavailable.

Resume: shared ratchets now pass, independent approved DTO consumer and typed
partial fixtures are written. Run updated changed-crate Clippy and diagnostics
tests, then connect L's accepted framing when its head is announced. Captures
and the one full native gate remain owed.

### Connection lifecycle and provider ownership

A connected notice is a new observation generation even when the book name and
NodeId are identical. The feature message now has Answered and Connected forms;
a Connected notice invalidates the previous row and rejects its old result.
The existing shared connection_ready seam remains the one producer; its owner
is supplying a standalone seam rather than importing another feature stack.
A test covers same-node reconnect before and after completion, then a fresh run.

Provider-specific diagnostics belongs to the providers package (catalogue row
568). Its unused source placeholder/words were handed to that owner and removed
from this package's final UI and copyable sections. The host report's approved
allowlisted cached Agents section remains part of ADR 0074, with no provider
discovery or agent.status call.

Resume: typed consumer compile check is queued. Connection message contract is
written/tested in source; shared lifecycle producer is pending its owner's seam.
The lead line now includes accepted Diagnostics localization at f2cb8cb6; merge
that accepted integration and preserve its copy-notice/language contract.

### Accepted localization integration

Merged the lead's accepted localization integration `f2cb8cb6` (including
Diagnostics 205f1e01). README append blocks both remain. In the owned view
module, the expanded scanner/collection remain and L's words_tests registration
is retained. The owned app State keeps the new collection plus L's copied_notice,
copy handler and owner-checked language refresh. The copy boundary now accepts
ReportText, so arbitrary raw text cannot bypass its limit/redaction.

L's language tests keep every exact report/clipboard/notice/input assertion.
Only fixture initialization and the moved report-field access use the typed
collection API; the copy call wraps the same fixture in ReportText. Existing
settings framing now reads the accepted cached diagnostics catalog, while new
collection framing remains diagnostic-flow. All unrelated merged head logic
and tests remain unchanged.

### Actual connection-ready producer

Merged the isolated connection-ready producer 1af642c6 directly based on the
announced app base. It changes only nodes/refresh plus its plan, and keeps
kept-offer delivery before lifecycle notices. The diagnostics entry is the
sole addition to its typed notice array; no second lifecycle hook or generation
registry is introduced. Its author's dependency source precheck is pending;
this package owns actual correlation tests and the combined native gate.

The source-only producer dependency received V1's no-source-blocker precheck
for provisional consumer reuse. The consumer regression now opens two real
framed in-memory NodeClients with identical node identity, runs the actual
connection_ready fanout, observes Connected before an equal Fleet, and checks
that a delayed old response cannot restore its copyable facts. This does not
claim the notice proves Read authority or a retained healthy connection; the
actual read still owns refusal and timeout outcomes.

### Validation resume

L accepted the final EN/KO wording and catalog shape (no app/security/gate
approval implied). Golden tests now cover every Problem recovery in both
languages and every progress status, including partial coverage. A canonical
complete typed fixture also exercises successful full coverage independently
from the existing partial-source fixtures.

The typed-consumer narrow submit b7a9d390 was never registered due to the build
service's global cleanup-lock stall. Its owner confirmed the exact absent SHA
and authorized one fixed-SHA resubmission after repair. That infrastructure
interruption is not a compilation/test failure. Current source still owes
updated narrow Clippy, scoped diagnostics tests, captures and the final gate.

The renderer destructures every DiagnosticReportFields member exhaustively:
adding a future safe source must trigger a consumer integration, rather than
silently omitting it. The app log-location line says not collected; it does
not claim that no app log exists when paths are deliberately omitted.
No unreviewed diagnostics producer or process-metrics WIP was consumed.

Resume: final consumer/source coverage is written; semantic metadata omits
paths explicitly. Product source is frozen for scoped tests and capture build.
The sole fixed-SHA retry of the interrupted prior narrow is registered and
queued; no duplicate is running. Current code needs its own changed-crate
Clippy/test evidence after that prior snapshot finishes.

The existing screen composition is split into host/source cards, report body
and actions so no long renderer owns unrelated page and dialog composition.
This changes no labels, controls, layout values or data decisions. The compact
settings registration converts its existing boolean into a local layout enum
for the extracted card helpers; no second control renderer is introduced.

The authorized retry of b7a9d390 completed compilation and found only two
redundant type qualifications in the collection tests; those are corrected.
The current head additionally includes the accepted localization and actual
connection-ready producer, so its own narrow check/test outcomes remain owed.
The capture fleet uses the immutable, hash-verified reference binaries and
matching baseline helper; it exercises backward-compatible node.info only.
It cannot prove a new daemon diagnostic RPC, which is still not consumed.

The first visual probe verified a working basic report on two isolated paired
fake nodes (negotiated protocol 15) and the adaptive viewer at 390x844. A saved
64-byte book label wraps within the compact viewer; Copy/Refresh stay visible.
This is a development probe, not final exact-head capture evidence. The report
now states its requested host count explicitly, independently of the saved-book
count, so a one-host retry cannot look like a missing host in an all-host report.
A scope-count regression preserves private label exclusion and zero-host
report/clipboard identity.

The next changed-crate narrow compiled the updated typed consumer/UI and found
one redundant Hub qualification in the new in-memory lifecycle test. Corrected
without changing that test's assertions or protocol. No functional runtime
failure is implied by that lint result; scoped execution is still queued.
