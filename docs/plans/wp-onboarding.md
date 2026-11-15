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
  only. Full daemon/process/log producers remain explicitly unavailable
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
Local app renderer and preference persistence facts stay available. No log path
is collected. Every unsupported section is named and final output is bounded.

## Shared base and ownership

The consumer merges announced app bases 8d5513e4 and 6036eeb8, accepted L/G4/G1
integration f2cb8cb6, and the isolated connection-ready producer 1af642c6.
Unrelated shared-head behaviour and assertions remain intact. Registration
uses link command/event/dispatch and shell/events. No size allowance is raised:
the demo/catalog extraction from the accepted integration keeps demo at 517
product lines after the union, below the incoming base's old record.

The existing shared `connection_ready` hook delivers kept offers first, then
one bounded typed fanout including Diagnostics. It runs after the connection's
existing harness reads and before sessions/client insertion/final Fleet. It
proves neither retained-client health nor Read authority. The consumer pins
`Machine.node` before dispatch and refuses another admitted identity. A new
Connected notice retires even a same-node observation; shared fleet identity
replacement, disappearance or offline state also retires old facts. No second
connection hook, identity registry or generation counter is introduced. Usage
may append its own fanout entry; integration must preserve the union.

Provider-specific diagnostics is A9 (G3/W22), not this package. Its placeholder
and three unused words were handed off and removed. The host report's safe
cached Agents source remains ADR 0074 data, with no provider discovery call.
Onboarding belongs to W15. No onboarding code is changed.

## Safe report consumer

`DaemonSnapshot` retains the approved immutable DiagnosticReport root. Its
identity must match the admitted connection independently of node.info. A
partial report preserves successful sources, observation times, unavailable
reasons, saturation and omission. Missing values do not become zero. The
technical format uses English headings and canonical JSON for nested safe DTO
rows, avoiding a second schema. Exhaustive field destructuring forces future
source additions to be integrated deliberately. Fixed home redaction is `~`.

Only cfg(test) fixtures populate full/partial daemon reports at this base.
The product uses node.info only, projecting NodeId, negotiated protocol and
closed lifecycle. It neither calls agent.status (refresh=false can execute
builtin discovery) nor borrows session/terminal/git logs as daemon logs.
N1's proposed process/pending RPC additions are not consumed before their gated
handoff. No remote daemon-success claim follows from DTO fixture tests.

A run admits at most 64 hosts, with four nonqueued link permits and a named
10-second node.info deadline. Combined UTF-8 output is limited to 128 KiB,
checked before and after redaction and after every host. Overflow refuses the
whole report, rather than silently truncating it. Host book labels stay in UI
rows; the copied report uses ordinals and states requested host count separately
from the saved book count. Run tickets reject duplicate/stale/closed results.
Only settled bounded ReportText crosses the clipboard boundary. Refusal clears
the local report too, so the screen cannot claim that the client was collected.

## Localization and copy contract

Existing Diagnostics framing uses L's accepted cached `diagnostics` catalog.
New framing is `diagnostic-flow.ftl` in EN/KO, with typed getters; there is no
parsing or formatting during draw. L accepted wording and key shape on c2000083:
EN SHA256 26910921b34440bb10aae10b119b159b09c932066126abe6ef25d92d1f406f2e;
KO SHA256 d41995ab1268eda265debfd33e8f79fccdab1c6c17dee961922da13f156d0b0f.
That review is wording/data shape only, not app, runtime or privacy approval.
Golden tests cover every Problem variant and every Progress status in both
languages. L's copied-notice ownership check and exact report/clipboard/input
retention assertions remain; only fixture initialization/access changed to the
new typed Collection and ReportText. Technical report data is not retranslated.

## Validation and current limits

On the previous base, narrow-7 Clippy passed for view/link/app and 28 diagnostic
tests passed (5 link, 23 view). The new base's first narrow found three redundant
type paths and a shared sidebar test missing the new node field; both corrected
without weakening its assertions. The current source needs its own final narrow,
scoped tests, exact-source captures and one whole native gate before handover.
Format, native size/boundaries and token checks passed after the base merge.

The isolated two-node capture fleet uses immutable hash-verified reference
binaries and the matching archived helper, never real agents. Its protocol-15
node.info exchange proves backward-compatible basic collection only. Captures
and local builds use a private target, XDG/HOME/config, authenticated Xvfb and
UID 1000. An interrupted pre-base capture rebuild is not final evidence. No
production files, permissions or service state are changed.

Current functional difference: full daemon and desktop diagnostics, extended
process metrics and daemon raw-log viewing remain explicit Unavailable. The
screen does not substitute fake producer success. Native watchdog/mobile shims
and bundled web hosting are outside this assigned app diagnostic sheet scope.

## Mandatory base rolls during package validation

The first whole native gate on 54478e20 passed 1250/0/1. The announced base3
6c782cb6 merged cleanly as43679f8d, with an empty native-source diff. The
coordinator explicitly ruled (message395a7209) that this whole stands with
base3's own green root/native gates and merged narrow Clippy/30 diagnostic
tests; no second whole was needed for that root-only union.

Before its local capture rebuild finished, announced base4 3cd0d3fc added the
accepted .19/.20 native packages. Its only conflict was the shell render
registration: both settle_diagnostics and terminal_present remain, in that
order after settle_settings. No private terminal logic is changed. The
obsolete base3 build was stopped only by its verified private process group.
The base3 ruling is not claimed as approval of this later native-source union;
its merged checks, capture artifact and closeout ruling remain to be recorded.
