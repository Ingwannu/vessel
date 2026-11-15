# Settings: Sync (milestone 12, G4)

## Decision log

- Purpose: inspect shared instructions, skills and tool-server names,
  secret-name presence, and the last reports received by a fleet home.
- Constraints: no join, leave or force-sync RPC exists. A member does not
  expose its local applied state or identify its home in the returned node
  list. Secret values must never reach this screen or its logs.
- Alternatives: infer the home from node order, show disabled actions,
  carry raw configuration into rendering, fail the entire read when one
  section is refused, or use a long-poll generation as a live member state.
- Choice: independent bounded reads, a names-only projection before the
  event boundary, explicit unknown states and local command guidance with
  working copy controls. Refresh reads again; no write is issued.
- Why: each visible fact corresponds to a field that exists. Successful
  sections survive another section's failure without inventing absence.
- Costs: the page describes last-read reports, not an atomic transaction
  or a member's current filesystem. Local commands still perform setup.

## Modules and flow

`view/src/sync.rs` retains the original web fixtures and their compact
outputs. `sync/page.rs` scopes reads with the already-existing parsed
machine key and non-reused tickets from Devices. `sync/snapshot.rs`
consumes a manifest into file metadata, tool-server names and secret
references; it combines partial replies without guessing roles.
`sync/rows.rs` interprets file kinds and member/connection status.

`view/src/text/sync.rs` and its modules contain both languages. The
screen's prose uses full words and sentences; dates use the viewer's
explicit offset, to seconds without fractions. The app reads that offset
once when its UI state is created. The existing size formatter supplies
binary units using the page's KB/MB/GB spelling.

`link/src/sync.rs` uses the selected existing connection for:

- `fleet.manifest`, immediately projected into safe names and metadata;
- `fleet.members`, preserving each device identity, label, applied
  version, conflict path and report time;
- `fleet.nodes` and home-only `fleet.peers` (the latter is also a
  connection-list fallback);
- `secret.list`, which returns names, never values.

Each call has a fifteen-second bound. At most eight page reads are in
flight, including their sends to the bounded event queue. Full/closed
command queues return a typed failure immediately. Only safe error codes
cross the event/log boundary, never an untrusted peer's message.

`app/src/areas/settings/sync.rs` keeps only UI handles, scope and offset.
Its `frame`, `shared`, `members` and `commands` modules draw the kit's
existing settings sections/cards/rows, badges, alerts, spinner and copy
button. The frame and shell have registration/dispatch seams only.
There is no new dependency or parallel settings control.

## Source comparisons

Paseo has no Sync screen. The comparable status layout followed is
**`screens/settings/host-page.tsx`**, particularly `HostStatusBadges`,
`HostConnectionError`, `HostSettingsPage` and `ConnectionsSection`:
short semantic badges, adjacent small alerts, section/card/row facts,
and explicit unavailable states. Its settings styles and the existing
native kit supply geometry and tokens; no reference asset was copied.

The feature/source comparison is with `backend/m11`'s
`web/src/features/settings/sync-section.tsx` and `web/src/data/sync.ts`.
The original `web/src/data/sync.test.ts` inputs/outputs remain tests in
`view/src/tests/sync.rs`. Added tests check full English and Korean
sentences, partial reads, privacy, scope races, local dates and presence.
This is a source comparison, not a running web-page capture.

Deliberate differences:

- No tool commands, arguments, environment/header literals or URLs are
  displayed; only names, as the coordinator requires. The web page kept
  raw transports and displayed command/URL details.
- All returned file names are shown, rather than dropping paths after
  twelve. Long names and paths wrap. A path, secret name, suffix or CLI
  command has its own fixed-width element, never a line break glued to
  prose in the body face.
- Behind counts are full sentences beneath a short status badge.
  Last-report time is named honestly; it is not called last sync.
- A failed publication does not erase stored secret names, member
  reports or connection data. A failed secret list means unknown
  presence, never missing; unread references never imply local-only.
- A home is identified only when a home-only call succeeds. A returned
  node list contains no home/role tag; its order is never used as proof.
- No mutating Sync action is drawn, including a disabled placeholder.
  Actual CLI commands are separate copyable values with sentences
  naming which machine and data directory they affect.

## Wire gaps (unchanged; no node or wire edit)

Join, leave and "sync now" are not on the wire. There is no sync-now CLI
command either; members pull automatically while running. Local
`vessel fleet init`, `invite`, `join`, `leave` and `status` are guidance,
not app-executed actions. `fleet.report` is a write and is never called;
`fleet.watch` reports generations, not live member state, so this page
uses an explicit read-only refresh instead.

The RPC does not expose a member's local applied state, applied-at time,
conflict reasons, role or home identity. `fleet.members.reported_at` is
what the home last received. `fleet.nodes`/`peers` give identity, name,
connection version and reachability, with no home tag. The local status
command is where the person checks those member-local facts today.

## Privacy checks

A UI event can carry `Published`, not `Manifest` or `McpTransport`.
That projection has no command, argument, URL, header, environment or
secret-value slot. The selected secret list is `Vec<SecretName>` only;
there is no value-read call. A hostile literal marker in every transport
position is absent from the projected snapshot's debug representation.
A separate link test proves a peer cannot echo a value through its raw
error. RPC response delivery and these read methods do not log payloads.
Runtime fixture values and literal-marker checks are recorded with the
capture evidence; only pass/fail results are printed.

## Fixtures and verification

`settings-fixtures.py` adds a current long-name member and a Korean
unreported member alongside the existing behind/conflict report. Reports
are synthetic, loaded before the fake node starts, with checks for all
three states. The fixture also checks a larger instruction file, an
executable skill, names-only HTTP/stdio servers, and stored/missing/local
secret-name shapes. The second node remains unconfigured by default;
optional `empty` and `files-empty` actions select instructions-only and
server-only homes, respectively, with empty member and secret lists.
The server-only home names an empty skills directory: the node requires
an instructions or skills source even when only servers are shared.
`standalone` restores the unconfigured shape. A configuration naming neither source is refused and cannot stand in
for a successful empty publication.
These are fixture setup, never app actions.

The small Devices followup keeps every revoke button in place, disabled
while sending; rows retain their layout underneath the open sheet.

No physical phone or real member sync loop is implied by these fixtures.
The ordinary account owns the scratch nodes and build outputs; token and
secret files remain owner-only. No production service or permission changes.

## Integration seams

- `app/src/areas/settings/mod.rs`: register Sync state, visible section and
  renderer; pass the existing window through to the kit's copy control.
- `app/src/shell.rs`: checked dispatch helper, reply arm and pre-render scope
  reconciliation.
- `link/src/lib.rs`: one command, one event, one bounded-call hub and dispatch.
- `link/src/web.rs`: a typed unsupported response for the browser demo.
- `view/src/text/mod.rs`: register the language table. The README records the
  section registration and fixture setup; no kit file changes.

The canonical published instruction path is `instructions.md`. Its file-kind
regression case is kept alongside the original page's `AGENTS.md` fixture;
classification never rewrites a returned path or a member's reported path.

## Gate evidence

The final native Rust tree passed the pinned whole-workspace formatting,
clippy with warnings denied, and 378 tests plus doc tests. The ordinary
debug build passed as well. Builds used the shared build-slot wrapper.
The main Rust workspace and fake-agent crates were not changed by this
slice; no new root-workspace gate is claimed. The Python fixture passed
its syntax and shape checks against the disposable nodes.

The final capture set has 112 frames, all inspected: both languages and
both themes at 1280 and 390, including neutral loading, failed reads,
empty file/server/member/secret lists, long names and paths, exact report
times, and four narrow pending-revoke confirmations. Five command-copy
controls were checked against the private display's actual clipboard.
Each pending revoke completed against the real scratch node after it
resumed; only the disposable inert fixture was restored afterward.

Runtime logs were checked against the actual fixture secret/token values
and the transport-literal marker in private memory; none appeared. The
screen model and hostile-marker tests establish the value-free rendering
boundary, and every final frame was inspected. No pairing link was
created for this capture pass. All owned app/display/fleet processes were
stopped and reaped. No physical-phone, browser-demo or real member pull
loop validation is claimed.

## Review correction: missing local-change reports

The node defaults an absent member report to an empty conflict list.
`KeptEdits::Unreported` now distinguishes that default from a known zero;
either an applied version, a report time, or actual conflict paths is
required before describing a count. Nonempty paths survive partial replies
without a version or time. Both the screen's full description and the
legacy row helper use that one rule. Whole English/Korean description
tests cover each source of evidence; the original web's known-report
fixtures remain unchanged. The native fmt, whole-workspace clippy and all 380 tests passed, followed
by the normal debug build. Twenty affected member/connection/exact-tooltip
frames were retaken and each opened: both languages, themes and widths.
The original capture set is the baseline; these frames replace its affected
states. Runtime log privacy and private-file ownership/modes passed again;
all owned app, display and fleet processes were stopped and reaped. No
root workspace code is included in this correction.

## Coordinator review correction: settings row composition

The failed-home row had an area-owned control wrapper with `flex_1` and
`items_start`. It competed with the kit's growing text column and painted
its badge at that second column's leading edge. The wrapper is removed:
the home name is the row title, its normal role explanation is the one
hint, and its badge is the sole trailing control. An unknown home keeps
a normal role hint and badge; the publication alert alone states the
failure and next step. No shared row geometry was changed. The regression
measures this production composition in GPUI with short and five-sentence
hints: equal text left edges, equal badge right edges at the padded edge,
and vertically centred controls despite different row heights.

A member now has one compact version/comparison/report-age hint. An absent
report has no hint and one neutral badge. Partial time/path-only reports
keep their known facts and say that the version is unreported; they do
not falsely claim that nothing was reported. Conflict paths appear only
in the following alert. That alert introduces and lists only paths, then
explains the received copy location in one sentence with the suffix inline
in the kit's code family. GPUI's inherited font-family override preserves
the alert's size, colour, wrapping and Pretendard fallback; no new control
or parallel text style was added. Whole bilingual row/partial-report,
relative/local-date, badge and notice tests cover the presentation rules.
The legacy web-fixture helper still distinguishes absent counts from zero.

Commands consume the lead's existing `settings::code_row` from `d050e7e`
(local dependency commit `cec9656d`), with its existing copy button as the
row's sole trailing element. The kit's four-point title/value spacing
replaces the area's separate wrapped control and twelve-point gap. Labels
are shorter complete instructions, with the command beneath its title and copy at the
row's right edge. No command is executed by the app.

Source comparison: Paseo `styles/settings.ts` (`row`, `rowContent`,
`rowTitle`, `rowHint`) and `screens/settings/host-page.tsx`
(`HostStatusBadges`, `ConnectionsSection`, `ConnectionRow`); the local
changes notice follows `components/ui/alert.tsx`'s description slot. The
reference has no fleet page; the necessary additional facts follow its
single-hint row structure. The original source comparison remains a
source comparison, not a capture of a running web page.

Verification of this combined correction: the native workspace's fmt and
whole-target clippy passed, all 386 tests passed (including the production
home-row GPUI geometry regression), and the normal debug build passed.
The gate logs explicitly show the changed view and app crates being
recompiled; no preserved-time source copy or clean workaround was used.
GPUI test support was already a kit test dependency; the app enables that
existing feature only for its geometry test, with no new runtime package.

Forty affected Sync frames and twenty-eight Devices frames were captured
from that build and individually opened: both languages, themes and
widths. The narrow member capture was scrolled one step earlier so the
first member is fully visible below the fixed header. All five command
copy controls were checked against the actual private-display clipboard
at both widths. Eight pending revokes kept the underlying rows in place
and completed after the scratch node resumed; only its inert fixture was
restored. Fixture checks and runtime secret/token/literal-value absence
checks passed again under the fleet owner's ordinary account; credential
files remain private. No invitation was issued. All owned app, display
and fleet processes were stopped and reaped. No physical phone, browser
or live member pull-loop validation is claimed. The pending wire actions
and caller-scope work remain separate until N3's reviewed delivery.

## Pending V1 followup (work in progress)

V1 returned the combined correction for two remaining surface problems.
Connection metadata now takes only a protocol version; reachability is
said once by its badge. Whole English/Korean expectations retain the
version for both reachable and unreachable fixtures, without a full stop
after the bare value. The Devices followup described in its own notes
shares this work-in-progress checkpoint.

Only formatting has been checked for this followup. Its full native
gate, normal build, production bounds test execution and affected
captures are pending; the earlier 386-test result and 68 captures belong
to the preceding correction and do not verify these edits. The attempted
lead integration was aborted after an intermediate lead head did not
compile. No unfinished merge is retained. Resume by integrating the
latest lead-reported gated head, preserving both Machines and Sync
registrations and initializing the Sync hub in Machines' actor fixture.
Then run the whole native gate and replace the affected connection and
narrow device frames before resubmitting to V1. This checkpoint is not
a UI pass or merge request.
