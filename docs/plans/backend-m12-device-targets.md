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
Pair device and Sync stays hidden. The native SDK is still protocol 15:
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
