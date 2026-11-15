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
