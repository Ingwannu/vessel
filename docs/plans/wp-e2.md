# E2: Pairing and devices

## Existing assets and choice

Modify the existing pairing area, Devices model and screen, connected-client
owner, secure `vessel-client` pairing, shared settings rows, dialog, Alert,
status badge and QR renderer. Do not add a second client, route registry,
credential store, modal renderer or QR implementation. The host registry
package supplies the one typed multi-route/upsert API, keyed by authenticated
NodeId and pinned key, and its direct/SSH screens; pairing consumes it after
confirmation. A mutable display name is neither a routing id nor authority.
Standard Rust, existing workspace dependencies and the existing bounded link
channels suffice. QR uses the already reviewed qrcode dependency in view.

## Reference map

Paseo 0.15.4 is the read-only behavioral reference, pinned at `1c00b365`.
Its source is read as a specification, with no source or assets copied.

- `desktop/components/pair-device-section.tsx:36–124,151–179,183–235,
  258–317,319–401`: connected/disconnected/loading/failure/unavailable,
  relay consent, retry, offer, QR, copy feedback and security warning.
  The query is stale after five minutes; QR correction is M, margin one;
  copied feedback clears after two seconds; the QR tile is 304 points.
- `components/add-host-method-modal.tsx:19–48,69–168`: direct, desktop SSH,
  mobile-only camera (omitted on F-Droid), then paste; one adaptive sheet.
- `components/pair-link-modal.tsx:101–205,210–275`: required/invalid input,
  saving guard, cancel clearing secrets, same-host password continuation,
  and target changes discarding password state.
- `hosts/host-confirmation-sheet.tsx:19–66,70–107` and
  `runtime/host-confirmation.ts:47–80`: public identity, fingerprint and
  relay detail rows, new/changed connection explanation, cancel/connect,
  exact-match bypass, one outstanding question and stale-answer refusal.
- `runtime/host-runtime.test.ts:751–837,1630–1715,4220–4245,4647–4677`:
  replacement versus learned security pin, stale switch failure, old
  confirmation answer and removed-host pending probe are the binding cases.

The per-device grant list/revoke/details are Vessel-specific, through the
reference's existing settings rows and adaptive confirmation. Their earlier
source/visual checks remain separately bounded in the G4 plan; they do not
establish complete pairing or effective-permission coverage.

## Structure and data flow

View owns state transitions, request/confirmation identity, typed failures,
relative/local exact time, credential masking and QR matrix. Link owns the
existing authenticated client, admitted generation, irreversible retirement,
RPC capacity/deadline, node-reported self/scopes and safe code projection.
The app draws cached/localized facts using kit controls and captures immutable
expected identity in callbacks. The shared `Machine.node` is the single
NodeId projection; its generation belongs to the same machine snapshot.
Before an effect, the actor compares both the expected node and admission.
A semantic session-read refusal never manufactures a scope or disconnect.

The node relay half is ADR 0096, developed separately on the node line and
consumed only after the integrator's gated app base includes it. Existing
`device.self` and typed invitation NoRoutes already exist on the SDK-23 base.
Old/unavailable self means unknown authority and no guessed current-device id.

## Deliberate differences and limits

1. Vessel's one-use, expiring invitation and device credential replace the
   reference's reusable offer and optional password. No password handshake,
   field or fake continuation is invented. Links use Vessel's validated
   format, rather than accepting a different product's offer as compatible.
2. Creating an invitation is explicit, with ten-minute validity; its text/QR
   starts hidden, is revealed for at most thirty seconds, and hides on copy,
   focus loss and departure. The reference displays a cached offer directly.
   This is the existing credential security floor. Only synthetic model
   output may be kept in a revealed-state capture, never a node-issued link.
3. Actual node errors cross as closed codes/details and whole EN/KO recovery
   messages. Raw errors, keys, routes with secrets and credentials never enter
   a screen/log/export through this boundary. A configured route is not proof
   of remote reachability, and neither a bind address nor a failed read is
   inferred to be a public route or permission verdict.
4. Desktop camera actions follow the reference's Electron platform: no mobile
   scanner or F-Droid camera stub is drawn as if it worked on this desktop.
   Mobile scanning and a physical phone are not verified by desktop captures.
5. Relay enable requires the operator-configured relay and explicit consent
   in ADR 0096; no hosted endpoint or secret is invented. Until the integrated
   call exists its action is absent. The reference has its own hosted default.
6. Host registry changes go through its single typed API. A redeemed one-use
   invitation followed by credential/book persistence failure needs an explicit
   partial outcome and recovery; it is never automatically redeemed again.
   Failed ordinary book edits leave the admitted connection intact.

## Resume point and verification boundaries

The retained normal Devices screen and its bounded prior captures are built,
but the E2 package is not passed. Shared HostErrors radius is separately
corrected/source-narrow reviewed. Shared dialog captured dismissal is a
small separately checked producer; its existence does not validate feature
policy. The admitted-generation producer074ade16..95c256bc received V1 bounded
SOURCE adoption and exact Link narrow pass (result docs72f081c3); its actual
shared-line integration is now present in c3429952. It uses the
canonical Machine.node field with no second NodeId map. Node-reported authority, full offer/method/confirmation
transitions and the host API consumption still need filling in.

Per-commit checks are narrow; security-sensitive commits receive source
review. One final package gate and changed-state captures follow completion,
with every kept capture opened, synthetic-only revealed credential states,
and explicit works / does not / unverified results. No permission, installed
service, production node or runtime configuration has changed.


## Effective caller authority: next vertical slice

[Decision Log]
- Purpose: device management controls reflect this admitted caller's actual
  effective grant; a denied session read must not hide usable Admin access.
- Existing assets: integrated protocol20 `GetDeviceSelf`/`DeviceSelf`, the
  shared exact-context unary adapter, Page tickets and existing device badge.
- Alternatives: infer privileges from reachability, device labels, a successful
  list, or a connection-global second cache. None establishes the admitted
  caller's grant and each can survive the wrong lifetime.
- Choice: a Page-owned, context-and-ticket-correlated authority read through
  the same bounded device adapter. The node remains the authority for every
  operation; no new wire API, dependency, resolver or actor is added.
- Why: existing `device.list`, `device.invite` and `device.revoke` all require
  Admin; only an explicit reported Admin grant exposes those controls.
- Cost: old/unsupported/failed self reads remain unknown, not a denied or
  default Admin verdict; show a whole recovery sentence and explicit retry.

A new authority read clears any previously owned list, details, invitation
and confirmation before dispatch, but is unavailable while an invitation or
revocation is already sent and pending. Changing Context clears its grant and
caller id; old Context/ticket replies cannot restore either. A known empty
grant is distinct from unknown. Admin without Read may manage devices; Read,
Operate, Sync or Relay alone do not imply Admin. The current-device badge
uses only the node's optional exact caller DeviceId, never a name match.
App callbacks recheck both current Context and reported Admin before creating
intent; the actor still fences the immutable original client. Server-side
permission changes remain authoritative and a successful self query is not
a guarantee that a later mutation succeeds.

Pure view transitions/tests come first, then the bounded adapter and screen.
Tests must cover stale authority replies, same-node reconnect, failure versus
known empty grant, Admin-without-Read and independent non-Admin scopes, caller
id absence, and clearing an already revealed synthetic invitation on refresh.
Final package captures/gate remain owed; no authority-flow pass is claimed.

The first authority source checkpoint adds the Page-owned `Authority` model
and six pure regressions: explicit Admin without Read, independent scopes,
unknown versus failed versus known empty, caller id/ticket replacement,
same-node reconnect, malformed duplicate/overlong facts, and clearing every
owned surface including an actually revealed synthetic invitation. It has
no screen or RPC wiring yet; therefore existing controls do not yet consume
this grant and no effective-authority protection is claimed from this step.
No product output contains the synthetic invitation. Narrow View checks are
next on the remote builder; final feature review/gate remains pending.


The first remote attempt at69d45683 was interrupted while queued by the
coordinator's build-VM restart (signal143 at10:35:42); it compiled nothing.
After the coordinator's authorized one-time replacement and the gated2e95b7d3
merge8fb011c88, exact8fb View Clippy found a panicking-slice lint in the
duplicate-scope check. No tests ran and that result is red. The check now
uses only bounded iterators; every original assertion and the five-scope
contract bound remains unchanged. New authority-read result booleans are
must-use. The next narrow batch proves only the pure View step, before
installing the separately prepared RPC/screen wiring.


### Additional queued-delivery boundary (not covered by source95)

W12 identified that an old result can wait in the App event queue before a
later retirement/Fleet notice. Context and ticket equality alone cannot
observe the actor's already-retired private watch during that interval. This
is an additional consumer requirement, not a demonstrated exploit or a
rewrite of the bounded95 source disposition. Devices still owes this proof
for public data, invitation installation/reveal/copy and rendered callbacks.
No current authority-flow or queued-credential UI pass is claimed.

W12 owns one proposed opaque delivery-validity capability from the SAME
SelectedConnection/original client and irreversible watch, under the lead's
shared-file writer order. Actor-only construction, private fields, redacted
Debug, no serde/raw-client escape/cache/new epoch, and exact Context checking
at installation are required. The producer is not ready or reviewed yet.
Devices will consume it rather than make a parallel validity mechanism;
matching pending work must settle honestly even if no newer snapshot arrived,
and an older binding cannot retire its successor. Before-effect/post-await
checks remain required, with no network/filesystem atomicity or replay claim.


## Authority adapter and screen source checkpoint

Exact47cd76fd pure View device tests passed52/0/0 after View Clippy0;
these remain prior-source evidence, not a result for the following wiring.
The gated dd13ce37 line is merged, preserving the shared producer/lock.

The device adapter adds a correlated self query through the same bounded
Hub and SelectedConnection.call. Page-owned authority now has a private,
validated report wrapper and a read-only accessor; a malformed scope set
cannot be constructed as a known report by an app caller. Refresh clears
all old management facts/intent, including an actual revealed synthetic
invitation. The screen asks for authority first, hides management for
unknown/non-admin grants, and uses only the reported caller DeviceId.
An Admin grant without session Read remains usable; a failed session read
never proves a scope. Existing controls recheck context and reported Admin
at dispatch, choice and confirmation. Failure/retry text is whole Fluent
copy in EN/KO, tested for every closed code and local reason.

New framed tests cover exact caller facts, Unsupported without a fallback
grant, and retired-before-poll self requests. The area callback regression
covers Admin without Read, refresh and removal of prior management intent.
These new source cases are not yet compiled or passed. The requested next
narrow suite compiles/tests View, Link and App; final package gate/captures,
actual authority flow and permission changes remain unverified.

The already documented queued-delivery gap remains open: these replies
still carry Context/ticket only. W12 owns the staged canonical weak guard;
this checkpoint neither fabricates that guard nor claims queue/credential
installation safety. No new kit control, resolver, SDK or dependency is added.
Dialog actual-close focus adoption and pairing/book partial-outcome wiring
remain final-package work, separately from this authority checkpoint.


The first adapter/screen narrow at7eb62220 returned red on exactly two
unnecessary Scope qualifications in View fixture helpers. Those qualifiers
are removed without changing any assertion or product byte. Tests were not
submitted after the failure. The log shows actual View/Link/Kit checking;
App checking did not complete, so no three-crate success is claimed.
The correction remains unchecked while the coordinator's isolated relay
lifetime security package is prioritized; queued Unary guard adoption still
uses W12's exact reviewed staging only after the lead grants its writer slot.
