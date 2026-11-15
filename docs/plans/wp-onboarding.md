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
admitted connection identity. Missing sources never become zero values. The
legacy synchronous report helper remains for compatibility/tests; this sheet
uses client_report plus Collection and only copies the admitted ReportText.

## Differences and producer boundary

ADR 0074 deliberately replaces raw daemon logs with scoped typed failed-call
observations, and excludes paths/environment/agent output. It has no public
node.diagnostics request at this base. Provider diagnostics, OS process lists
and daemon tails likewise have no current callable safe producer. Screens
state this; they never display a fixture result as a live successful report.
Local app renderer and preference persistence facts stay available. No log path
is collected. Every unsupported section is named and final output is bounded.

## Shared base and ownership

The consumer merges the announced app bases through frozen base4 3cd0d3fc,
accepted L/G4/G1 integration f2cb8cb6, and the isolated connection-ready
producer 1af642c6.
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

## Current-payload clipboard admission

[Decision Log]
- Purpose: an old rendered Copy closure retains an old ReportText after a
  refresh/close; an old enabled button must not bypass run invalidation.
- Existing assets: modify the existing private copy boundary and its real GPUI
  tests. Standard PartialEq suffices; no new control, API or dependency is needed.
- Alternative: rely only on the next disabled button render. That leaves the
  earlier frame's callback able to publish its retained payload.
- Choice: compare against the current settled Collection report before writing
  or announcing a copy. This protects every caller while retaining L's notice
  ownership and language behavior. Equality work is bounded by 128 KiB.
- Evidence: the real handler preserves a sentinel clipboard after close,
  refusal, a pending replacement and a different completed replacement. The
  second L fixture explicitly admits its report and proves copy before another
  notice replaces it; no existing report/clipboard/input assertion is removed.

## Frozen base and gate ruling

The original whole on 54478e20 passed 1250/0/1. The mandatory base3 union was
native-source identical and received a narrow-only waiver. Base4 then added
accepted native packages, so that waiver does not cover it. The sole conflict
was shell render registration: settle_diagnostics and terminal_present both
remain after settle_settings; terminal implementation is unchanged.

Coordinator ruling db62f27e explicitly required one additional whole native
with priority and froze E4 on base4 through capture, review and handoff. No later
base or node gate40 is consumed. The stale-copy fix is within this frozen base.
The final executable-source commit is 50f0ee0bbd46bf75ae878e9e546d11a44c85e7b9.
The prior base3 build was stopped only by its verified private process group.
The final capture build acquired its ordinary slot before a separately approved
one-build priority switch, so that local exception was never used.

## Validation and material limits

Final whole native: wp-onboarding-20261115T090841Z-50f0ee0b, every step rc0,
1302 passed / 0 failed / 2 ignored. Logs: /tmp/work/wp-onboarding/whole-native-base4.
The step set includes embedded fonts, format, size/boundaries/tokens, all-target
Clippy, application build and the complete native test set. No owned root crate
or web source changed; inherited base4 root changes have the coordinator's
separate green root gate, not a new W20 root claim.

Narrow evidence: merged base4 view/link/app all-target Clippy rc0 on 999dde6a;
diagnostic/copy filter 33/0/0 there (view25, link5, app3 including an unrelated
fork-language match). The final app copy-boundary Clippy is rc0 on 50f0ee0b and
its exact Settings Diagnostics GPUI filter is 3/0/0. The unchanged view/link
cases cover identity, reconnect, ticket supersession, independent partial
sources, bounds, closed failures and every EN/KO Problem/Progress sentence.

Local final capture build completed in 7m18s; the exact binary SHA256 and tracked
native source hash are in /tmp/work/wp-onboarding/package-manifest.json.
The matrix is /tmp/work/wp-onboarding/captures/50f0ee0bb: EN/KO, dark/light,
1280x900, 390x844, 800x600, page/report plus scrolled page bottoms. Wide captures
were moved to window origin so the PNGs match 1280x900, rather than the initial
2px screen-edge clip. Smaller captures were already exact. The extra observed
running screenshot is retained as 1278x900 and is not a matrix-size claim.
Extra before/after one-host Refresh and offline report images prove retained
scope and independent failure. All 32 required-size images and four additional state images were opened,
inspected and hashed in capture-manifest.json.

Actual X11 clipboard probes report only byte counts/hash/absence booleans. All
hosts: requested2/node-info2; one host before/after Refresh: requested1/node-info1;
after disconnecting one fake: requested2/node-info1 with an explicit offline
status. Fixture routes, token/link prefixes, book labels and local paths are
absent. The probes do not print or persist raw clipboard contents. The broad
matrix includes the later offline fixture in corrected wide shots; smaller
shots show both connected, all using the same executable source.

The isolated fake fleet uses immutable hash-verified reference root binaries and
the matching archived helper, never real agents. Its protocol15 exchange proves
backward-compatible node.info only. It does not prove a new daemon producer.
All runtime work uses UID1000, private HOME/XDG/config, authenticated Xvfb250,
private book/auth mode600 and fixture directories mode700. No production files,
permissions or services are changed. The app, both fake host parents and Xvfb were stopped only after UID/executable/
start-time verification. No fixture parent remains live; cleanup.json records
the actions. No foreign job was stopped.

Current functional difference: full daemon and desktop diagnostics, extended
process metrics and raw daemon-log viewing remain explicit Unavailable. Full
and partial daemon report fixtures are cfg(test) only. Provider diagnostics is
A9 and onboarding W15; no duplicate work is landed. Native watchdog/mobile
shims and bundled web hosting are outside this app diagnostic sheet package.
