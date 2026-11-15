# Devices: bind a request to the connection that supplied its facts

## Contract before implementation

[Decision Log]
- Purpose: a device row, confirmation and invitation belong to the actual
  machine that supplied them, not just a mutable name in the local book.
- Existing assets: `BookEntry` already has complete typed equality;
  `NodeClient::node()` supplies authenticated node identity. The link owns
  one `Connected` map. Devices already has non-reused request tickets,
  confirmation guards and redacted credentials. E2 has no separate admitted
  generation or scope cache and confirmed no active connection/edit overlap.
- Constraints: a stable Online(alias) survives final-only reload today;
  session-list failure removes a client even when authentication succeeded.
  Neither a name nor a failed read establishes identity or authority.
- Alternatives: compare aliases, compare NodeId alone, reread/reconnect from
  every screen, or create another connection actor. Names can be rebound;
  NodeId alone misses an observed A-to-B-to-A/reconnection; extra actors
  duplicate the connection and can disagree about it.
- Choice: extend the existing connected-client record with its parsed book
  binding and a monotonic admitted generation. A per-connection validity
  watch retires queued work. Publish safe connection facts independently
  of session reads. Requests and their rendered callbacks carry the actual
  NodeId and generation; dispatch checks that exact admitted context.
- Why: one actor remains the owner, the physical client is immutable once
  selected, and a context change can invalidate the pure view before a frame.
- Costs: small forwarding hooks in connection refresh and successful book
  edits, new pure identity/reducer tests and transport-adapter regressions.
  No new dependency, wire call, schema, kit control or second client map.

## Reference and scope

Paseo 0.15.4 `screens/settings/host-page.tsx:409–442` captures server and
connection identity for removal, rather than looking up a display name at
confirmation time. `desktop/components/pair-device-section.tsx:36–69`
selects its client/query by serverId. `runtime/host-runtime.ts:2669–2670,
2876–2891` likewise looks up a stable identity. Vessel additionally needs
the local-book/admitted-generation guard because a book alias can be edited
to target another node. This is a security difference, not a new visual style.
The existing settings rows, adaptive dialog, destructive control and short
hints remain the presentation reference already documented in the G4 plan.

Base: gated lead `d30804b2`, merged without conflicts as `7ed69812` on the
G4 branch. It adds the current Connections navigation; Devices remains
Pair device and Sync stays hidden. Historical base status (superseded by the package-base integration below):
the native SDK was protocol 15:
fleet controls exist, but `device.self` and typed invitation NoRoutes are
not integrated. Those calls wait for the lead's node integration; G4 never
merges the node branch directly. Effective scopes stay unknown until the
node reports them; a failed session read never becomes a scope verdict.

## Rules the tests must establish

1. Each admitted connection has a non-reused typed generation and actual
   NodeId. Reconnecting to the same node produces a different context.
   Counter exhaustion refuses admission; there is no wrap or clock identity.
2. Refresh compares the saved parsed entry before reusing a cached client.
   An observed changed/missing entry and reload retire old contexts before
   slow work or new facts are published. Successful rename/forget retire
   the affected context; failed edits leave it intact. Default-only edits
   do not invent a new physical identity.
3. The view clears old rows, read-only details, confirmations and invitation
   text/matrix when its selected context changes, before drawing again.
   Its request-ticket source survives that change. Stale callbacks capture
   their rendered context; they cannot reinterpret the same DeviceId on
   another connection. Stateful element identities also include context.
4. A revoke confirmation has its own non-reused intent, captured by its
   primary action. An old confirmation cannot consume a new one. Sending
   remains disabled and a context switch never silently retries it.
5. List/invite/revoke requests and replies retain context plus request
   identity. Before RPC admission, the link compares the expected context
   with the existing client and its active binding. An already retired,
   replaced or closed admission is refused without sending a device call.
   No action is redirected to the client now found under the alias.
6. Once admitted to an immutable client, a call may have an effect on that
   confirmed node even if context changes before its reply. Its late reply
   cannot populate another context and it is never automatically replayed.
   This does not promise cancellation of an already sent remote mutation.
7. An authenticated client and its identity facts survive a semantic read
   refusal. Devices does not derive transport death or effective scopes
   from session-list reachability. A disconnected transport retires its
   context. Existing session-list error presentation is not redefined here.
8. Validity watches, queued operations and reply sends share the existing
   bounded call lifetime. The concurrency and deadline values get named
   constants used by production and tests. No extra polling loop, unbounded
   task set or mutex map is added.
9. New refusal words are whole English/Korean Fluent messages selected in
   view. Existing credential, raw-peer-error and QR redaction stays intact.
   A retired invitation is never put in a field, log, file or kept capture.

The observation boundary is explicit: book changes admitted by the actor
and connection replacements are covered. This does not claim an atomic
filesystem/network transaction or detection of an external A-to-B-to-A edit
whose intermediate state was never observed. The invariant remains that
the confirmed physical client is never replaced by another alias lookup.

## Verification and resume point

First pure view rules/tests; then the existing link actor and its framed
RPC test peer. Tests cover changed node under the same alias, same-node
reconnect, observed A-to-B-to-A, stale replies and callbacks, failed versus
successful edits, no-Read/refused session data with usable device RPC, and
retirement before/after RPC admission. Use events/barriers, not sleeps.
The affected UI states are captured once at package end. Under A14 commits
receive narrow checks, with one final A13 package gate. Security changes
retain per-commit review. No implementation or integrated-flow pass exists
yet; the prior normal Devices and Alert producer passes stay separately
bounded in `backend-m12-g4.md`.

### First implementation checkpoint

The pure `devices::identity` module now carries alias, authenticated NodeId
and a non-clonable epoch source. Six tests cover changed node, same-node
reconnect, observed A-to-B-to-A, unchanged facts, alias rename, retirement
and counter exhaustion. This is explicitly staged: no actor or page consumes
it yet, so no runtime protection is claimed. The existing assets are extended;
no external API, dependency or second registry is introduced.

Narrow local short-lane checks: `cargo test -p vessel-app-view
 devices::identity::tests` passed six tests; changed View compiled. Then
`cargo clippy -p vessel-app-view --all-targets -- -D warnings` passed.
Logs: `identity-view-test-01.log` and `identity-view-clippy-01.log` in the
track's resumed evidence folder. No whole gate or captures were repeated.
V1 accepted the preceding contract at `d751095d` as contract/source only;
barrier tests and integrated enforcement remain pending.

### Integration under verification

The identity source now lives once on the existing link actor. `Connected`
holds its immutable client, parsed binding, public context and irreversible
validity watch. The existing refresh module forwards only admission/retirement hooks to
`link/src/connections.rs`; no second actor or client registry exists. All
observed replacements retire before a new dial is awaited. The shared Machine snapshot pairs welcome NodeId and epoch independently
of the sessions result; no separate NodeId-facts event is retained. A known closed device call retires its context; a
semantic read refusal does not. Successful rename keeps client, agents and
follow while issuing a new alias/admission; failed edits do not retire.

Paseo's additional `host-runtime.test.ts:751–837` cases replace a re-paired
client but retain a learned, already-enforced security pin. Vessel mirrors
that rule for an absent book id becoming the actual authenticated NodeId
with every other field unchanged. Removing/changing the pin, routes or
credential source retires the admission. This does not claim detection of
changed contents inside an unchanged credential-file reference.

Page binding clears rows, details, invitation and confirmation before a new
frame; its ticket source remains. A distinct typed `RevokeIntent` separates
confirmation identity from RPC tickets. Production callbacks reconcile
navigation before using their captured context, and stateful row/primary
button identities include admission/intent. The link checks context before
spawning and checks retirement/transport closure before polling the RPC.
An already sent mutation may still take effect on its original node; it is
never replayed or redirected. Replies are reduced in view only for the same
admission and outstanding request. New refusals are cached whole Fluent
messages per action in both languages. No new dependency or wire change.

The existing framed machine-book peer was extracted once into `testing/peer`
with its old default behavior and a controlled request/reply mode. E2 cleared
that test-only seam. New tests use peer barriers and the actual screen
methods with a detached bounded link; no production node or real agent.
Narrow verification is still in progress. Exploratory compile failures are
retained: the first rejected an underscore in a Fluent file name; the second
found moved imports, test visibility and checked-date arithmetic. Neither is
a pass or a final frozen check. The final package gate and captures remain
pending, as do node-reported scopes and typed NoRoutes from the lead's B0
integration. Existing normal-flow and Alert producer passes are not extended.

The live confirmation now stores context, intent and device in one immutable
record with private fields, carried unchanged into Sending. There is no
parallel optional intent that could describe another selection. Even an
accidentally retained old confirmation reattached to a newer page refuses
confirmation; its dialog is not drawn. The public intent is opaque and its
allocator remains the page's one non-reset ticket source.

The fourth narrow probe reached the app and found the private-shell bridge,
unsent reply envelope, two callback arguments and a naming/qualification
lint; those failures remain recorded. The link's test-only extraction also
needed literal `cfg(test)` boundaries (in addition to the platform gate),
so Clippy recognizes fixture helpers as test code; no lint is disabled.
The current changed-crate tests, Clippy and capture build run sequentially
in the coordinator's newly authorized ordinary lane, each bounded at 600
seconds. No whole local gate or duplicate queued build was started.

The fresh CLI-paired fleet's complete start/check exited zero. A separate
Admin-only stored-grant fixture probe restored its original grant but is
inconclusive: its first Sessions connection hit the unchanged three-second
dial bound, while the following Devices read succeeded. This is retained
as a failed probe, not proof of a permission refusal or a native UI flow.
The adapter's deterministic typed-Forbidden peer regression remains the
separate source-level case; no real-node NoRead UI pass is claimed.


### Package-base integration

The coordinator's package base `8d5513e4` was merged cleanly as `0f35054a`,
with all Devices work first preserved as a hashed external patch plus new
source files. The restoration used current timestamps. Registration now
uses link command/event/dispatch modules and shell/events; the existing
refresh module keeps refresh/connect, while connections owns only admitted
metadata, reconciliation and facts. The root is off its old size list;
no size record was raised. The new base's UnassignedProtocol refusal and
its regression were preserved when resolving the one restoration conflict.

This base integrates protocol 23, including `device.self` (assigned 20)
and typed DeviceInvite NoRoutes. The earlier SDK-15 deferral is historical.
The boundary now maps only Conflict + that exact NoRoutes detail to the
existing setup explanation; Forbidden, missing detail and future reasons
retain their code-only fallback. Raw message text never selects a branch.
The earlier decoded-offer route guard remains for older servers. Effective
scope/self presentation still awaits the following E2 sub-feature; nothing
is inferred from reachability, names, device rows or successful reads.

The current identity work is a security checkpoint within the newly assigned
E2 pairing/devices package. Per-commit narrow checks/source review remain;
the one whole native gate and changed-state captures belong to the final
E2 package, rather than another whole gate for each checkpoint (A14).


### Shared Machine projection and package base 2

Gated package base `6036eeb8` was merged cleanly as `0b0c2c43` after the
narrow probe ended. All in-progress work was preserved outside the repository
as a hashed patch and complete source copies, restored with current timestamps.
The probe's five test-only lint findings remain recorded; they were not a pass.

The lead's `Machine.node` is now the only NodeId projection. The same Machine
snapshot carries a serde-defaulted typed `connection_epoch`; legacy/demo facts
without both cannot supply a request context. No separate NodeId map/event
is retained. A generic `view::connection` module owns the one allocator and
context vocabulary, with legacy Devices re-exports. Session-read Forbidden
retains welcome identity/admission even though Reach still reports that the
session read failed. Rebind, remove and reload retire before a slow dial.

All selected unary calls share one exact-alias preflight and before-poll
validity/actual-client check in the existing connection owner. Closed reports
are submitted through a weak actor command, rather than published from a
background task: only the actor orders retirement with Machine snapshots,
so an older snapshot cannot be queued after that retirement and revive it.
The weak sender does not keep the actor inbox alive. Request failures retain
structured peer detail only inside link, with redacted Debug/Display, before
the feature maps them to safe codes. Every answer keeps its original context.

The long-lived guarded follow/subscription entry requested by the workspace
owner is private test-staged, pending W12 product wiring; no unguarded NodeClient is exported to a
screen. The final package gate and integrated scope/alias/UI proofs remain
owed. Shared dialog return-focus is a separate producer change, with actual
close acknowledgement, pending/stale refusal and newer-focus preservation.


### Base 4 shared preflight checkpoint

Gated package bases 3 (`6c782cb6`) and 4 (`3cd0d3fc`) are merged, the latter
as `d68646306`. The source uses the lead's Machine.node and existing kit
controls. Restoration conflicts retained the new shell fields and tightened
the demo size record to 515; no size record grew. Source/API ownership and
consumer obligations are in `backend-m12-connection-preflight.md`.

Narrow attempt `targets-core-clippy-12` returned 101 because base4 newly
embeds three missing Geist Mono faces; no actor/App pass is claimed. Those
three ignored font assets were copied with current timestamps after checking
the existing pinned Geist1.9.2 font script matched. Hashes, paths, ownership
and mode are in `base4-font-assets.json` (UID/GID1000,0644). No executable,
service, credential or permission policy changed. Earlier waiting ordinary
probe11 was explicitly cancelled before compilation for the coordinator's
shared-seam priority ruling; it is not a failed compiler result.
