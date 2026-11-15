# Native Settings: Devices and Sync (G4)

## Decisions and scope

[Decision Log]
- Purpose: bring the page's paired-device and fleet views into native
  Settings, with Paseo's settings rows and pairing credential warning.
- Existing assets and constraints: track E already ported the page's
  devices/sync decisions and fixtures; the lead owns the kit and pairing
  with a machine. The node and wire are outside this track's scope.
- Alternatives: another settings frame or control; invoking the local CLI
  to manage a remote machine; storing a revealed link in a text field.
- Choice: extend the existing view, link through the typed client, draw
  through the existing settings and dialog kit. Keep invitations only in
  memory and project their text only while explicitly revealed.
- Why: one layout and one credential policy across screens; a remote
  action cannot safely be implemented as local filesystem edits.
- Costs: node calls remain the limit of what this page can offer.

## Slice 1: Devices view logic

The page's `web/src/data/devices.test.ts` cases are retained: scope words,
last seen/paired ages, revoked filtering, trimmed optional invite labels.
Additional cases cover initially masked links, expiry before the 30-second
limit, re-reveal, masking, debug redaction, stale list/invite answers, and
revocation requiring confirmation. Requests have non-reused typed tickets;
leaving a machine invalidates pending state rather than accepting its late
credential on another page. No GPUI dependency or I/O is introduced.

Paseo `utils/time.test.ts` supplies the English age cases as well: floor
minute/hour/day units, just now below a minute, and calendar dates from
seven days. Settings writes those ages in full words, with exact viewer-local
instants to the second and a named UTC offset on hover. Each row sentence is
a complete translation unit; numeric counts reach its language match.
Korean interpolated names need no particle chosen by the implementation.

The screen uses the existing settings card/row and adaptive confirm.
Requests and replies have one link enum seam; rendering has no network I/O.
Captures and final gate results are recorded below after visual checks.

## Sync contract gap (source inspection)

`vessel-cli/src/fleet.rs` deliberately implements `init`, `join`, `leave`
and `status` by local data-directory access through `vessel_node::fleet`,
not through the RPC connection selected by `--node`. The current RPC list
has no remote join, leave, sync-now or member-role/status operation.
`fleet.manifest` is a home-only read and reports Unavailable for a member.
The existing page reads manifest, members and secret names only.

Missing for the requested remote actions: a role/home/status reply for the
selected node (home identity, applied generation, local conflicts), join
with link and explicit grant/key confirmation, leave, and sync-now with
an applied/failed result. The coordinator was informed; this track changes
neither node nor wire and does not draw controls that do nothing.

## QR encoding and drawing

The native view adds `qrcode` 0.14 (the main workspace's version), with
`default-features = false`: MIT OR Apache-2.0, one encoder crate, no
image/SVG/rendering dependencies. The kit's `qr.rs` draws only a supplied
matrix, dark-on-light in both themes with a four-module quiet zone and
integer pixel modules. Encoding is guarded by `Pairing::visible_link`;
there is no matrix in hidden, expired or copied states, no serialized
matrix and no debug representation of its cells. The ISO/IEC 18004
Annex I vector checks the encoder-to-matrix conversion cell by cell.

Revealed-state checks use a test-only invitation and its QR matrix in
`view/src/tests/devices.rs`; normal-build captures show masked fields
only. This was chosen over an app capture feature, so no compile-time
switch can replace an invitation in an installable build. The release
script needs no exception: there is no such app feature to enable. No
issued link or live QR is retained in a capture.

## Failure handling and bounds

Device RPCs run beside the fleet task, with eight in flight at most,
a fifteen-second deadline and the existing bounded command/event
channels. The permit covers the reply send so backpressure does not
accumulate completed tasks. Node refusal codes are retained; arbitrary
error text is discarded at the credential trust boundary. A hostile-peer
regression test checks that echoed credentials reach neither the view's
error nor its Debug/Display output. The page does not optimistically
remove a device; it reads the list after the confirmed response.

## Fake fleet shapes

`settings-fixtures.py`, invoked by `fleet.sh`, seeds inert device rows in
the disposable database: many rows, a long name, Korean, never seen,
seen before, and revoked. Its random hashes cannot authenticate a known
credential. The fake home has instructions, long/Hangul skill paths,
MCP server names and one present/one missing secret name. A synthetic
member report remains behind with two kept edits. This is a stable
rendering fixture, not proof of a real member's sync loop. Missing device,
revoked, language, home or behind/conflict shapes fail the fixture check.


## Reference comparison for Devices

Paseo's `desktop/components/pair-device-section.tsx` supplies the QR tile,
copyable link and password warning below the link. Its settings headings,
rows and spacing are the existing native kit's. The page's Devices and
RevokeDialog supply per-machine ownership, scope/age facts, revocation's
explicit consequence, and the initially-hidden/30-second/copy/blur rule.

Deliberate differences: the node's invitation expires in ten minutes and
works once (Paseo's offer can be cached longer); the page's masking policy
is retained as a stronger credential display limit. Revoked entries are
filtered as on the page rather than shown as active devices. Ordinary
paired rows have no badge; a never-seen row says Not seen. A current-device
badge requires a proven identity, never a guessed live connection state.
An explicit Refresh also lets devices paired outside this app appear
without minting another invitation. Read and action failures use localized
code-based words instead of an untrusted node message. Loading is a plain
sentence because the native kit has no skeleton control. No relay toggle
is drawn: the node offers no corresponding configuration RPC.

## Problems encountered

The first cold gate exposed normal compile/lint fixes (the QR's checked
layout arithmetic, explicit Arc cloning, request-to-command ownership,
and splitting invitation rendering into small functions). No lint or test
was weakened. The fake skill was initially excluded because it lacked
required frontmatter; the fixture now has valid frontmatter and is checked
against the node's resulting bundle rather than counting source files.

## Review fixes and limits

A successful invitation can contain neither a direct address nor a relay.
That cannot connect another device, so the link boundary returns the typed
`NoRoute` problem, with localized setup and retry guidance, instead of
showing a credential that cannot work. Tests cover zero, direct and relay
routes. No wire or node code changed. An old reveal timer also carries
its unique request ticket: equal wall-clock deadlines and clock rollback
cannot let it hide a subsequent reveal.

The badge's exact-value test now checks warning and error tints in both
themes as well as success. The kit control and mapping remain unchanged.

Comparison here is a **source comparison**, not a running page capture:
Paseo's pairing section, status badge, theme values, time utility/tests
and language strings; and the Devices page/data/revoke-dialog sources
and data tests read with `git show backend/m11:web/src/...`. An earlier
reference-page capture attempt was stopped when the coordinator clarified
that boundary; its processes were stopped by PID and it supplies no
evidence here. The fake node/CLI and the native captures use only this
track's own two target directories.

The shared Alert is a separate prerequisite commit (`85cc00b`). Devices
uses its small warning below the link and copy controls, and its error
notice beside a failed list, invitation or revoke. List retry is the
notice's working action. The pure view chooses warning/error meanings;
no screen chooses a colour or kit variant. This replaces the earlier
plain warning before any final slice captures.

The accepted Alert's two follow-ups are in this slice: geometry constructs
named fields in each size arm; its body-indent arithmetic states the sum
and the workspace's `clippy::float_arithmetic` and
`clippy::arithmetic_side_effects` restrictions beside the calculation.
No lint suppression was added and no geometry value changed.

Device dispatch also checks command admission: a full or closed link queue
settles the correlated pending state with a localized failure. The general
link send path only logs rejection; using it here could leave a request
loading indefinitely before the RPC timeout ever starts. A bounded-queue
regression covers full and closed queues. This is local to Devices and
changes no other area's send API.

## Validation

The final pinned native workspace gate passed: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, and `cargo test`. The test
run passed 4 app, 12 kit, 13 link, 229 view, and 73 page-contract cases
(331 total), with doc tests passing too. The main workspace's source is
unchanged; its CLI, node and fake harness binaries were built only to
run this track's disposable fleet. No main-workspace gate is claimed.

`settings-fixtures.py check` passed against the running own-built fleet.
This is a source comparison with the reference screens/tests, plus native
runtime evidence; it does not claim a running web comparison or a scan
from a physical phone. The normal app has no fixture/capture feature.

Visual review caught the loading badge taking the card's whole width
because it was a block child. A flex wrapper now preserves the badge's
intrinsic width, like the reference's short label. The final loading
captures are retaken after this correction and the whole native gate.

The normal app's Copy action placed a real disposable invitation in the
private display's clipboard, and the copied-state capture shows both
credential text and QR masked with the localized copied confirmation.
The link was read only into process memory and piped to the own-built CLI;
no argument, log or file held it. That CLI attempt timed out after the
node created the device record but before a credential file was saved.
A bounded retry with the app closed completed pairing and authenticated
successfully with the new device credential. The earlier timeout remains
recorded without attributing its cause. No node/wire change or work on the
lead's adding-machine flow was made to work around it.

The confirmed revoke action then marked that disposable device revoked
on the node; all fifteen other active fixture devices remained active.
The screen refreshed from the authoritative list. Cancel/Escape captures
kept the original long-name fixture intact. No physical phone scan is claimed. The retry's saved credential permits
a separate post-revoke authentication check.

Phone review also showed why the page names the selected machine in the
pairing hint: its section heading is no longer visible after a long
scroll. That destination stays beside the name field in both languages,
with whole-sentence tests. The form captures are retaken for this change.

The saved retry credential authenticated before revocation and was refused
explicitly as revoked after the UI's confirmation. Both selected test
records are revoked; all fifteen stable active fixture devices remain.
The credential file is mode 0600, its directory 0700, owned by the same
ordinary account that ran the test. No production permission or service
changed.

Final visual evidence: 48 normal-build frames cover empty, failed, loading,
long, confirmation and hidden-link states in both languages, both themes
and 1280/390 widths. Four desktop hover frames originally retained UTC instants; the review
correction below supersedes those frames.
Every final frame was personally inspected; the folder's manifest records
its hash and masked state. Runtime copy/revoke frames are kept separately
from that 52-frame set. No issued QR or link is in either set. The first
automation pass preceded the first painted frame; it was replaced after
checking navigation, not counted as a successful screen check.

The reference's busy state uses the kit's animated spinner as the leading
element of a neutral badge, with a localized label. Revoke feedback is the fresh authoritative list
and an inline failure notice, without adding a separate toast system.
Shared helper candidates for the lead: MachineKey/Target, non-reused
Tickets, safe Problem codes, and the toolkit-free Age thresholds. They
remain in this area's view module rather than guessing E's unpublished
Machines helpers.


## Devices screen review correction

The desktop lead's branch was merged first, keeping every track's glyphs
and the accepted badge/alert followups. Its spacing commit is the reviewed
scale; this screen now uses those constants and one kit touch target.
The merged native gate and the required main-workspace fake-agent checks
passed before the merge commit was finalized.

The view now takes an explicit viewer offset for both old calendar dates
and exact hover instants. The app reads that offset once from its own
system clock. Exact instants show seconds and a named offset, never the
node's fractional storage representation; changing the viewer's zone
while the app runs takes effect on restart. New whole-sentence tests cover
UTC, positive, negative and fractional-hour offsets and local date rollover.
The original Paseo age fixtures still use an explicit zero offset.

Invitations show whole minutes remaining, rounded down, then an under-one-
minute sentence and the existing expired sentence. A pure deadline helper
returns the next wording boundary; the app timer holds only the public
expiry and stops when the invitation changes or the page leaves. It never
reads or changes credential visibility. Boundary tests check each wake,
the minute counts and the final expiry in both languages.

The public verification key is its own fixed-width element below the
instruction. Failed actions use their alert once, without a duplicate
failure badge. Listing, making a link and revoking use neutral badges with
the kit spinner. Korean now uses invariant particles naturally, and the
confirmation body describes the consequence without repeating the long
name already present in its title. Credential gates and node calls are
unchanged. Sync remains parked until Devices is accepted.

The accepted typed destructive button is included from the schedules
track's shared control commit. Revoke uses its own foreground and no
screen-local colour override. The confirmation retains only the device's
public metadata while the request is sent; its destructive and cancel
buttons are disabled with no handler, and a neutral spinner shows progress.
The existing ticket correlation and all credential policy remain unchanged.


The correction's full native gate passed: formatting, clippy with warnings
as errors, 358 tests and all doc tests. The normal app build passed too.
Retakes cover seven states (failed link, long list, confirmation, pending
revoke, list loading, link creation and masked invitation) in English and
Korean, dark and light, at 1280 and 390: 56 frames. Four desktop hover
frames show local instants to seconds. A further masked frame shows the
countdown changing from nine to eight minutes after 101 seconds without
input or focus events. All 61 retained frames were personally inspected;
none shows a live invitation or QR. The viewer's system offset in these
runs is UTC+0; the other offsets are verified by the view tests.

Pending-state captures pause only the disposable home node, confirm the
revoke, then resume it and observe the device's tombstone. The unredeemable
long-name fixture is reinstated in the scratch database between runs so
all eight shapes use the same row. This is fixture setup, not evidence
that a revoked real credential can authenticate again. The earlier real
pair/authenticate/revoke/refuse check remains the separate runtime proof.
The fixture check still passes afterward. An initial pre-paint automation
frame was replaced; the remaining runs wait for visible content first.
A sparse pixel sampler initially rejected a painted phone frame and was
corrected; every resulting frame was judged directly, not by that sampler.

These checks run under the ordinary account that owns the scratch fleet.
Its token files remain 0600; the node book refers to those files rather
than containing tokens. No production service or permission changed. No
physical phone, QR scan or browser build is claimed by these retakes.


## Sync slice

The read-only Sync design, source comparisons, wire gaps, privacy checks
and evidence are in [the Sync notes](backend-m12-g4-sync.md). The Devices
row followup in that slice retains disabled buttons while its revoke
confirmation is open, preserving the layout underneath it.

## Coordinator followup: compact Devices rows

In the same pass as the Sync surface correction, a device now has one
access/last-use hint. Pairing history and every independent scope remain
in the existing exact tooltip, with viewer-local seconds and named offset;
the list no longer stacks three history lines. The access label is a
primary display summary, never an authorization decision. A read plus
sharing/forwarding grant is not mislabeled read-only. The fake fleet now
includes read-only and session-access rows with checks; `row-access` upgrades
only the named inert rows of an already-seeded disposable fleet.

Normal active devices have no badge: the heading already says they are
paired. Unseen/revoked facts and a proven caller identity have typed,
short distinguishing badges. The connection currently supplies no caller
id, so the app passes an unreported identity; it never identifies itself
by a device label. N3's pending `device.self` supplies that fact later.
The accepted m11 revoked filter is retained (V1 confirmed that conditional
badge examples do not request a history view). No revocation policy,
credential policy or node call changed. Tooltips keep Paseo's existing
no-touch behaviour; no new detail surface or parallel control was added.
The legacy web-fixture helper remains unchanged; screen tests now compare
the compact hints, sparse badges and full tooltip in both languages.

This followup shares the Sync correction's passed native gate (386 tests)
and normal build. Twenty-eight affected Devices frames were individually
opened: long and many rows, pending confirmations, and desktop exact
tooltips, both languages, themes and widths. Pending controls stay in
place and disabled. The fake-node revoke completed after each resume;
only the inert test record was restored. No invitation was created, no
production service or permission changed, and private credential-file
ownership and modes passed. See the Sync notes for the combined checks.

## Pending V1 followup: wrapped actions

The distinguishing badge/action group previously wrapped to the next
line's left edge on narrow rows. The area now shares one production
composition with a bounds regression: the existing kit button stays in
a natural-width, nonshrinking group with automatic leading margin.
It keeps the trailing edge whether beside or beneath the text; a
full-width group was rejected because it would move ordinary actions
below their names too. The shared kit's geometry and button appearance
are unchanged. The test covers badge present/absent and short/wrapping
names, measures the actual button edge, and checks badge/button centres.

This is an unverified work-in-progress correction: formatting passed,
but compilation, the bounds test and new narrow-screen captures remain
pending. See the Sync notes for integration/resumption requirements.

## Lead's finishing work (2026-11-13, after `1a91edee`)

The lead merged G4's last follow-ups (`1a91edee`, a `wip:` commit) into
`backend/m12-spike` and checked them on the fake fleet; G4 resumes from the
lead's head.

- Devices: a row is the device's name, one hint (its access and when it was
  last seen) and the revoke action; a badge only for an exception (not seen
  yet, revoked, this device). At 390 points a wrapped badge and button stay
  at the row's end (V1's second finding). Paseo has no list of paired
  devices (its settings have the pair-a-device row and modal only), so the
  row follows Paseo's settings row: title, one hint, trailing action.
- Sync, read as a whole against Paseo's settings conventions (Paseo has no
  Sync page):
  - a shared file's path is one line of a set width, cut in the middle
    (kit `fonts::elide_middle`), the whole path in a tooltip (it wrapped at
    its slashes over three lines);
  - a tool server's row is its name only (the same title on every row);
  - a secret's row is its name in the code face as the title, what uses it
    as the hint, and the presence badge at the row's end;
  - kept: a member's hint keeps "N behind home" beside its "behind" badge
    (the count is not in the badge); machine connections no longer repeat
    their badge in the hint (V1's first finding, G4's follow-up).
- A Sync read the link did not take settles as the devices page's does.
- Offline: both pages say the app's sentence, no transport text.
