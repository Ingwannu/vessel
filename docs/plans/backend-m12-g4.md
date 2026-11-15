# Native Settings: Devices and Sync (G4)

The earlier HostErrors producer/handoff incorrectly called `lg` 12. G2
and V1 resolved the actual reference token to 8; the radius-only correction
and its separate evidence are in `backend-m12-g4-alert.md`. The old green
gate proves the old source and is not corrected-source or visual evidence.

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

## Resume point (2026-11-15): device list and revoke

The branch merged the published app head `56a7cbd3` without conflicts
(fast-forward). All earlier G4 commits, including `c7636345` and the
`1a91edee` checkpoint, were already ancestors of that head. The lead's
finishing notes above describe the intervening completion; those changes
are reused rather than reconstructed. Settings already places this page
under the chosen machine as **Pair device**, with slug `pair-device`.
Sync remains hidden, and is not enabled by this slice.

Decision: modify the existing page and kit composition. The list, exact
local dates, semantic badges, disabled pending controls, typed destructive
confirmation, request tickets and safe RPC error boundary are reused.
No external dependency or kit control is added. The remaining problem is
that the full grants and pairing date were available only through hover,
which cannot expose them on touch. A row now selects public metadata for
the existing adaptive dialog. Its three ordinary settings rows show the
paired date, last-seen date (or an explicit absent record), and every
independent grant; Close has no destructive effect. The row's existing
Revoke action opens only its confirmation, with click propagation stopped.
Details and revoke selection share the tested view rule; stale, revoked,
offline and pending selections are refused before focus or network work.
A successful list reconciles the selected public record, and navigation
discards it. Nothing here guesses the caller's identity or privileges.

Source comparison, Paseo 0.15.4:

- `packages/app/src/styles/settings.ts:24–59`: the existing card, title,
  one hint and trailing control; values remain the kit's tokens.
- `packages/app/src/screens/settings/host-page.tsx:499–555`: the comparable
  named row and trailing removal action. Lines 398–493 retain a pending
  confirmation and disable its cancel/destructive actions.
- `packages/app/src/components/adaptive-modal-sheet.tsx:87–115,639–707`:
  the existing kit's normal adaptive dialog, including its standard
  desktop width and ordinary body/header/footer. No new dialog geometry.
- `packages/app/src/screens/settings/host-page.tsx:1095–1125` and
  `desktop/components/pair-device-modal.tsx:15–34`: Pair device's current
  navigation opens pairing. The invitation/QR surface is the next slice;
  this slice retains its existing route and credential policy.

Deliberate differences: Paseo has no paired-device list or per-device
revocation surface. Vessel's node provides them, so they use Paseo's
existing settings rows and adaptive confirmation. Read-only device details
are an additional surface required to make dates and all grants available
without hover; it uses the same dialog and rows, with no new control.
Long device names wrap, as the existing accepted Vessel page requires.
The original `backend/m11:web/src/data/devices.test.ts` filtering/age
fixtures remain;
additional pure tests cover details selection, pending confirmation,
removed/revoked records, navigation and whole English/Korean metadata.

The existing link admission and deadline are unchanged; this slice adds no
bound, node method, wire version, authorization policy or retry. A separate
bounds-only cleanup was deferred to keep the behavior commit focused.
The published base lacks `device.self`; caller identity/authority therefore
remain unknown instead of being inferred from a label or another device.

Current evidence: formatting, size, boundary and token checks passed. The
filtered native view Devices run passed 35 tests, with 680 unrelated tests
filtered out, from a cold build folder; view all-target Clippy also passed
with warnings denied. The initial app Clippy found one unused import; it
was removed and its narrow replay passed with warnings denied. Both logs
are retained. These are narrow checks, not a whole gate. No whole
native gate is run on this machine under the resumed build policy. The
second-machine native gate and its exact-commit result, a normal app build,
and fresh authenticated-Xvfb captures are still pending. Earlier captures
and gates do not validate these new details interactions. A temporary Xvfb
check accepted the authenticated client and refused one without its cookie;
its 0600 authority file and owned process were removed afterwards. Checks
run as the development account (UID/GID 1000), with no service or permission
change. Details use the kit dialog transition; idle CPU and real-GPU motion
remain unmeasured until the new build can be run.

Next: finish narrow checks of changed crates; commit this device-list and
revoke slice; run the supplied native gate on the second build machine;
verify list/loading/empty/refused/offline/long/details/confirm/pending and
completed revoke against the disposable fleet, in both languages/themes
and required sizes. Inspect every retained capture before V1 submission.
The old Diagnostics draft remains separately backed up and uncompiled.
The quality audit also identifies existing Devices/Sync target and queue
shape duplication; it is not expanded here. Consolidating it is a separate
behavior-preserving change, rather than a new shared framework in this
interaction slice.

## Self-review followup: one loading signal

The first authenticated captures exposed an older duplicate: the list's
neutral spinner badge said Loading, then a line underneath said Loading
devices (both languages). Keep one short badge and remove the repeated
line and its unused translation field. The section already names the
thing being read. Paseo's comparable host-page read progress uses its
activity indicator; the coordinator's accepted neutral badge and spinner
remain, with no second account of the same fact. No kit geometry changed.

The first source commit's remote native gate passed all six steps, with
1043 passed, zero failed and one ignored test. Its local normal build also
passed. That evidence belongs to the first commit; the loading correction
gets a new exact-commit gate and normal build before final captures.

Runtime fixture limit found during this slice: the published fleet script
reaches its final shell seeding check, where the old guard expects `url`
in a book now paired through the CLI. That check fails; it is not counted
as a full fleet pass. The two nodes were stopped, then their actual paired
direct routes were checked as loopback, outside the protected preview
ports, before restarting only those same disposable nodes. The Devices
settings-fixture checks pass. The failure is routed to the shell owner;
no fixture source or guard is weakened here. The app uses the same scratch
home as the CLI, so the paired book's home-relative credential files are
resolved correctly. Pairing remains a pipe, never a stored link.

## Device list and revoke: resumed verification, 2026-11-15

Source commits: `eadc3324` adds press-to-open public device details and
selection tests; `fe29f8f4` removes the duplicate loading sentence. The
coordinator's fleet guard fix `d30804b2` is taken as `a86e0cc9`, at the
clean boundary after the second source gate. It changes only the shared
shell-fixture script: paired direct/relay entries and legacy URLs are
checked without weakening the loopback and protected-port guard. The
actual paired fleet now passes that guard and the settings-fixture check.
The original full fleet start remains a recorded failure; it is not
retroactively called a successful end-to-end start.

Final A13 native run: `devices-20261115T032509Z-a86e0cc9`, evidence under
`/tmp/work/m12-g4/resumed-20261115/native-gate-03`. Fonts, fmt, quality,
all-target Clippy, build and test each returned zero, with no skipped step.
The ten test sections independently sum to 1043 passed, zero failed and
one ignored. `pairing_browser_words_capture` is ignored because it needs
its own private display/config/fake offer; this slice does not claim that
test or physical-browser pairing. Earlier exact-source gates are retained
separately. No whole native gate ran on the saturated local machine.

The local normal capture build (`local-app-build-02.log`, return zero)
recompiled the changed view, link and app crates at `fe29f8f4`; the later
Python-only guard fix changes no native Rust, manifest or embedded font.
There is no capture feature or alternative installable app. The own-target
CLI/node/testkit build also passed; root source did not change, and no
root-workspace gate or web-page verification is claimed.

Capture folder: `/tmp/work/m12-g4/resumed-20261115/captures`. All 178 final
original PNGs were opened personally, and their SHA-256 and dimensions
match `capture-manifest.json` (digest
`5255fa971be42128f2f11e38ef79436714871854f7ff0447f2ec2d2367237a15`).
The normal matrix covers English/Korean, dark/light, 1280x800, 390x844,
800x600 and 1024x700: loading, many/long rows, two details states, long-name
confirmation, pending and completed revoke. Each of its sixteen revokes
was confirmed in the real disposable node's store, then only the inert
fixture was restored. Eighteen more originals cover real GPUI scales
1.25, 1.5 and 2 for details/confirm/pending in English dark and Korean light;
the six additional real revokes were likewise checked. Existing kit glyphs
remain legible and the long titles, metadata and controls wrap without
overlap. Pending Escape leaves the confirmation open and its disabled
controls preserve the rows behind it. The source comparisons and necessary
differences listed above apply to these frames as well.

Refused and offline states each have the full sixteen-combination matrix:
the first uses an actual CLI-paired Read-only device; the second stops only
the verified disposable laptop node. Empty has the same matrix but is a
visual fixture: after admitting the disposable admin connection, the
fixture temporarily marks its disposable rows revoked directly in SQLite,
then restores every original tombstone in a finally block. It checks empty
list rendering/filtering, not the authorization behavior of API revocation.
No product feature, owner-token book or authorization bypass was added to
the app. Real API revocation is evidenced separately above. Exploratory
and failed automation frames are outside the final folder and not passes.

Idle CPU, five seconds per sample on software Vulkan/Xvfb: English dark
1280 list 1.80%, details 2.00%; Korean light 390 list/details 2.20% each,
as a percentage of one core. This measures the whole app, not only this
area; real-GPU animation, physical-phone input and other operating systems
remain unverified.

Privacy/permissions: no app invitation was created and no retained frame
has a live link or QR. CLI pairing went through an anonymous pipe only.
The metadata model can hold only public DeviceInfo dates/grants; no
credential field enters it. Existing RPC errors remain typed codes. A
private audit checked 165 retained logs for pairing URLs and the exact
disposable credential bytes in memory: zero hits, with no value printed.
All ten private books/credential/token files were UID/GID 1000, mode 0600,
under 0700 parents. The app, authenticated Xvfb and both owned node children
were stopped and reaped; the authority cookie, socket and pause marker
were removed, all fixture tombstones restored, and the scoped process
scan was empty (`final-cleanup.json`). No service account, service, live
configuration or file permission was changed.

Reproduction fingerprint: `native-source.sha256` enumerates `git ls-files
spikes/native`, selecting `.rs` and Cargo.toml/Cargo.lock/rust-toolchain.toml,
in Git's path order, hashing current file bytes. Its 666 entries have file
digest `3a0304a5783b93c6e5b001cc237bd284d52916af1fab80915734c233c5afee75`.
This following documentation-only evidence commit changes none of those
bytes, so it runs only the noncompiling fast checks, not another gate.

## Resume point after the device-list slice

Done: list, press-accessible exact metadata, one loading signal, confirmed
revoke, source gate and personally inspected final capture package.
Submitted status is review requested, not V1 approval or a merge.
Unverified/deferred: invitation/QR refresh and full pairing interaction,
device.self permission-sensitive controls on the integrated newer node,
Sync navigation/actions, physical phone/browser and real-GPU motion.
Caller identity/authority stay unknown on the published base; no self-by-name
inference. Sync stays hidden. The next clean-boundary assignment is the
coordinator's minimal typed shared Alert presentation for the schedules
partial-host-error banner; its first-use UI captures belong to its consumer.
The separately archived Diagnostics draft remains uncompiled and outside
the resumed slice.

Review disposition: V1 subsequently passed the bounded Devices slice
(`eadc3324` / `fe29f8f4` / `a86e0cc9`, docs `0e51fdbe`). It independently
matched all 666 Git input hashes, six successful gate steps, the raw
1043/0/1 test total and all 178 PNG hashes/dimensions. V1 personally opened
40 risk-selected originals, including all eighteen enlarged frames;
its visual pass covers those inspected states, not all 178 or every
platform/accessibility case. The author opened all 178. Its exact selection
is retained in `m12-review/g4-devices-a86-v1-captures.json`. Runtime privacy
and process cleanup remain the author's retained evidence, not V1's live
audit. A supplementary private check also parses credential envelopes to
scan their bearer field separately: twelve secret patterns, 166 logs,
zero matches. Coordinator screen comparison and merge are still separate.

## Shared Alert host-errors presentation: contract first

[Decision Log]
- Purpose and intent: the schedules consumer needs Paseo's partial-host
  error banner without painting another notice in its area.
- Existing assets and constraints: `kit/alert.rs` already owns one notice
  frame, five semantic variants and four density sizes. Default/ExtraSmall
  has vertical padding 8, no stack gap and muted text; Error adds an icon
  and an accent border. Neither matches the source. `status.red_300`,
  `Type::Meta`, the spacing scale and radius tokens already exist.
- Alternatives considered: an area-specific div, caller style overrides,
  a second renderer, or another semantic error variant. These duplicate
  ownership or conflate a presentation with the meaning chosen in view.
- Choice: modify the existing Alert. A private typed presentation chooses
  ordinary Notice versus HostErrors; both use the same frame. The narrow
  public entry `alert::host_errors(messages, tokens)` accepts one localized
  whole message per host. The kit owns their vertical stack and wrapping;
  it accepts no arbitrary colour, padding, icon or size override.
- Why: the partial-load banner has no title, icon, action or indentation.
  Its host messages are siblings with spacing 4, not one opaque description
  whose internal spacing the screen must recreate. The ordinary `alert`
  entry and every existing caller retain their behavior and defaults.
- Costs and effects: one additional named presentation and helper, no
  dependency, token, glyph, view semantic variant, wire or application change.
  The consumer remains responsible for whether it is shown and its words.

Reference: Paseo 0.15.4
`packages/app/src/screens/schedules-screen.tsx:334–347,400–417`
(`ScheduleHostErrorsBanner`, `errorsBanner`, `errorsBannerText`). Its frame
is transparent, neutral border 1, radius 8, padding 12 on every side, gap 4,
normal text 12 in palette red 300, with no icon. The outer responsive margin
belongs to the schedules layout and is not added to Alert. Compare also
the full `components/ui/alert.tsx`, whose default description and actions
must stay unchanged, and `styles/theme.ts:71–78`, red 300 `#fca5a5` in both
themes. No intentional visual difference is planned for this profile.

Verification planned: exact profile geometry/colour/type in both themes,
the existing variants/sizes preserved, then A13 native gate on the changed
committed source. G2 owns first-use captures with long messages, several
hosts, phone width, both themes and languages in its separate consumer
slice. A source/gate pass here will not claim those UI states passed.

Implementation: the existing `alert` entry resolves Notice and builds its
unchanged lead/description/action children; `host_errors` resolves HostErrors
and places each message directly in the same frame. The private enum has
no arbitrary style parameters. Existing five-variant/four-size exact tests
remain, with both-theme host geometry/type/red-300 checks and an exhaustive
ordinary-profile preservation check. No callers or semantic view enums
change. README names the additional presentation.

Resume point: Devices is with V1. The shared profile source is ready for
its own exact-commit A13 gate, not yet passed or consumer-ready. First-use
captures remain G2's responsibility; no alert UI/capture pass is claimed.

The first Alert run, `devices-20261115T035609Z-68fac599`, passed fonts/fmt/
quality, then stopped at Clippy (101): its Decision Log's `HostErrors`
name lacked Markdown backticks. Build/test did not run, and the failure
is retained under `alert-native-gate-01`. Pre/post source pins match all
666 native inputs. The correction changes that comment only, without a
lint suppression or behavior change. The corrected commit receives a
short Clippy check first, followed by a new whole A13 gate; no pass is
claimed before those results exist.

## Shared Alert result and next boundary

Alert source `68fac599` plus the documentation correction `eb75a637`
passed A13 run `devices-20261115T041805Z-eb75a637`, retained under
`/tmp/work/m12-g4/resumed-20261115/alert-native-gate-02`. All six native
steps returned zero, with no skipped step. Ten raw test summaries sum to
1045 passed, zero failed and one ignored (the existing pairing-browser
capture test). Both new profile tests passed; Clippy checked the changed
kit, and build/test actually recompiled kit and app. The preceding short
Clippy run `devices-narrow-20261115T041507Z-eb75a637` also passed in 90
seconds. The first failed run remains separate, not retried into a pass.

All 666 selected native Rust/Cargo/toolchain inputs match the pre-check
pin after the whole run. `alert-corrected-source.sha256` has digest
`3618e360f73f140e7fee4954d1975e5de54178f0751e149fa7af5b890a1e7e70`;
the file selection is the same rule documented above. `alert-final-audit.json`
records the independent sums and input comparison. This results-only
documentation changes no compiled input and gets no unchanged gate rerun.

Public API for review: `kit::alert::host_errors(messages, &tokens)`, where
messages implements `IntoIterator<Item = SharedString>`. One localized
whole message becomes one wrapping sibling. A private typed presentation
chooses its look; both entries share the same frame. No existing caller,
semantic variant, token, dependency or glyph changed. The only shared
documentation seam is the Alert paragraph in the native README. Source
comparison remains Paseo's schedules-screen lines 334–347 and 400–417;
there is no planned visual difference in this profile. G2's first-use
long/multiple-host/phone/English/Korean/dark/light captures are still owed.
This is a source/gate submission, not their visual approval or API handoff
before V1 passes it. No app, node or display was started for this control
slice; no service, credential or permission changed.

The Devices PASS above is bounded normal-flow evidence, not physical-node
binding or authority-sensitive integration proof. A separate source review
found that a stable Online(alias) can survive book replacement: reload
clears clients and emits only the final fleet. The current Devices target
can therefore retain rows/confirmation across an alias rebind. Next slice
must carry authenticated NodeId and an admitted connection/book generation
through snapshots and confirmed requests, compare at dispatch, and reject
stale replies and observed A-to-B-to-A changes before drawing an old
confirmation. No runtime reproduction or fix is claimed yet. A session-list
failure also cannot establish this device's scopes or physical reachability;
the no-Read flow needs an independent authenticated fact. V1 confirmed both
are outside its earlier bounded pass and required these regressions. Their
absence must not be hidden by the 22 successful ordinary revokes.

Resume point: Devices normal-flow slice is with the coordinator after V1's
bounded pass. Alert source/gate is now submitted separately to V1; consumer
captures and V1 approval remain pending. At this clean boundary merge the
coordinator's gated lead head, then write the next Devices identity/authority
contract and tests first. New node calls are consumed through the lead's
integration, never by merging the node branch directly into this tree.
Invitation/QR and Sync remain separate following slices; the old Diagnostics
draft remains archived and uncompiled.

Current boundary: V1 accepted the Alert producer at `eb75a637` with result
`a65317cf`, source and native gate only. G2 received the exact API/commits
and owns the first-use package captures, empty-list hiding, initial-load
surface and outer margins. The full gated lead `d30804b2` was then merged
cleanly as `7ed69812`; this newer combined tree is not covered by the older
standalone gates. A14 now makes a feature package the gate/review unit:
narrow checks per commit, one whole gate and affected captures at package
end, while security retains per-commit review. No unchanged merge-only whole
gate is added. The next security contract is
`backend-m12-device-targets.md`; its pure identity checkpoint `1f4a091a`
received V1 source-only acceptance. Actor/Page integration is now under
narrow verification; no integrated or visual pass is claimed yet.


### Blocking shared connection seam (base4)

The exact-alias/admitted-epoch producer and Devices compatibility wiring are
under narrow verification, not a feature/UI pass. The one actor/API and
consumer obligations are documented in `backend-m12-connection-preflight.md`.
The final E2 package still owes device.self permission presentation, pairing
parity, integrated captures and its whole gate; no historic normal Devices
result is reused as authority/alias proof. Shared dialog ReturnFocus is
separately source/narrow reviewed at11fba709, not this actor's approval.


### Current E2 authority checkpoint

The shared preflight range074ade16..95c256bc received bounded V1 source and
Link narrow acceptance and was integrated by the lead; its historical
limits remain recorded separately. The gated dd13ce37 line is now merged.
Exact47cd76fd pure View devices tests passed52/0/0 and View Clippy0, with
actual View compilation; this is prior-source evidence only.

The next authority RPC/screen source is described in `wp-e2.md`: explicit
self query, validated private report, whole localized failures, independent
Admin without Read, current-device id and management guards. Its changed
source still needs the requested narrow checks; queued DeliveryGuard adoption,
actual-close focus lifecycle, pairing/book partial outcomes, final captures
and the whole E2 package remain unverified. No old normal-flow screenshot
or shared producer pass is relabelled authority/UI evidence.
