# ADR 0096: Explicit relay consent from the app

Status: V3 contract pass at9021e3b8; implementation, protocol assignment and runtime verification pending.

## Purpose and existing assets

The Pair device screen must ask before enabling a relay and then make an
invitation through the existing device authority. Paseo 0.15.4
`desktop/components/pair-device-section.tsx:70–80,183–235` changes only
`relay.enabled`, then refreshes its offer. Its
`server/daemon-config-store.ts:272–281,377–384,416–420,762–769` filters,
keeps and applies that choice; the launch-override case refuses changes.
`server/daemon-config-store.test.ts:109–129,234–253,288–310` covers
persistence, failed application and an override-owned setting.
`server/pairing-offer.ts:28–46` returns no offer while disabled and otherwise
uses an operator endpoint or that product's hosted default.

Vessel already has one `Relays` owner, secure transport (ADR 0009), device
invitations (ADR 0010), and `node.settings`/`node.set_settings` (ADR 0038).
The node starts configured relay registrations from operator-supplied routes,
registration token and TLS roots. There is no mutable relay setting, no
Vessel hosted-default endpoint, and no safe reason to accept a new endpoint
or registration secret from a remote settings request.

## Alternatives

- A second relay actor or connection path would disagree with the existing
  registrations, callback links and shutdown tracker.
- A URL or token in a remote request would add a network/credential authority
  this consent does not need. Copying another product's hosted relay would
  also cross a service boundary without authorization.
- A restart-only flag or an acknowledgement before application would make
  the app claim that an offer is available before the setting takes effect.

## Decision

1. Modify the existing settings object with one optional `relay` field.
   Its safe read projection describes **configured registrations** as
   `unconfigured`, `disabled`, or `enabled` with aggregate link health
   (`connecting`, `online`, `retrying`, `stopped`, `unknown`); a disable still
   stopping registrations is explicitly `disabling`. It also says
   whether independent fleet callback links are present, without an address,
   count, key, token, ticket, diagnostic string or home-directory path.
   An absent field from an older node means unknown support, never disabled.
2. A change is the typed choice `enabled` or `disabled`; an unknown future
   choice is refused. Existing `node.settings` retains Read and
   `node.set_settings` retains Admin, with both nonforwardable. Ordinary
   reads and authentication never enable anything. There is no new method,
   URL input, credential upload, alternative transport or new dependency.
3. Enabling uses only the node's startup-validated, configured relay routes,
   registration token and TLS setup. With no configured routes it refuses
   `Conflict` plus a closed `RelaySettingsError::NotConfigured` detail.
   Stopping refuses `Conflict` plus `Stopping`; an unknown choice is
   `InvalidRequest`; safe storage failure is `Internal` or existing protected
   file `Conflict`. A peer cannot echo an endpoint or credential in these
   new messages. Existing unrelated settings error contracts are unchanged.
4. Persist the explicit choice through the existing owner-only atomic
   settings-file writer, preserving its unreadable/later-file safeguards.
   Validate the complete request before any write, including mixed parking,
   driver and relay changes. A failed write changes no live relay state and
   no sibling setting. Serialize through the existing settings mutation lock.
   No-op changes write nothing. Saved relay choice outranks the initial
   configured-registration default at restart, as ADR 0038 already does for
   parking; the routes and secrets themselves still come from startup config.
5. Extend the existing configured registration records with their own stop
   token and tracked task handle. Disable cancels those registrations and
   waits for them to stop before acknowledgement. Enable starts exactly one
   registration per configured route. A repeated enable never duplicates a
   registration. A rapid disable/enable cannot start replacements until the
   old tasks have ended. Startup waiting links obey the same choice before
   the gateway is served. Shutdown and settings changes share that owner.
   `RELAY_CHANGE_DEADLINE` is ten seconds, measured after mutation admission;
   the one existing settings mutation lock serializes changes. Timeout is a
   typed uncertain failure, never success. The stopping records stay owned
   and bounded until their tasks end, state remains `disabling`, and another
   enable refuses Busy rather than adding overlapping registrations. The
   caller rereads state rather than auto-retrying.
6. This switch controls configured registrations, **not** device revocation,
   direct routes, independent fleet callback registration, or already
   established encrypted data connections. These distinctions are deliberate
   Vessel-specific behavior and must be stated in the app's explanation.
   A callback remains governed by the existing fleet relationship and ticket.
   The read projection reports its separate presence so it cannot silently
   be called fully disconnected. No new fleet authority is granted.
7. Default device invitations omit disabled configured relay routes while
   retaining explicit direct routes. The existing typed NoRoutes refusal
   still runs before randomness and pending admission. A tracked fleet
   invitation's explicitly selected callback routes keep their existing
   authority. Disabling is not revocation: an already issued invitation or
   credential is not rewritten or automatically consumed; its lifetime and
   one-use rules stay unchanged. Re-enabling may make its route usable again.
8. `MAX_CONFIGURED_RELAYS` is sixteen routes, checked before starting tasks.
   Paseo has one configured endpoint; Vessel retains bounded multi-relay
   support. At most sixteen active or stopping configured registrations
   exist together; a stopping generation cannot overlap its replacement.
   Production and tests share the named constants, never UI literals.
   Old configured routes exceeding this limit fail startup explicitly,
   never truncate. No new
   polling, unbounded task set or mutex around network awaits is added.
9. Add one pending `Option<ProtocolVersion>` constant for the new field/error
   detail. The integrator assigns its number. Keep existing method rows at
   their original version/shape/scopes; a nonempty relay mutation makes the
   whole request require the field's assigned version, even in a mixed
   request. Before assignment it is refused before send/dispatch. New reads
   project the field away for older peers, including settings-write replies.
   An older decoder ignores unknown optional read fields; old nodes cannot
   silently acknowledge an unsupported mutation because the client gates it.
10. The app consumes the integrated, gated node line only. Until then the
    Enable relay control stays absent. New refusal copy is a whole EN/KO
    message selected from typed codes; raw peer text never reaches a field,
    log, clipboard or capture. The device credential and invitation remain
    governed by the existing masking, expiry and no-capture rules.

## Verification contract

Use only isolated testkit nodes and loopback relay fixtures. Reuse the
reference persistence/no-op/failure/override cases with Vessel's stated
saved-over-startup precedence. Prove Read/Operate cannot change a relay,
Admin can, both methods refuse forwarding, mixed requests are atomic, old
peers and pending fields refuse before effects, settings survive restart,
read projections and Debug contain no secret-bearing routes, and a canary
token/pairing link/home path cannot cross the new error boundary.

Barrier tests must prove no duplicate registrations on repeated/rapid changes,
write failure leaves registration unchanged, disabling waits for its own
registration tasks, direct sessions/device grants remain intact, callback
presence stays honest, invitation routes follow the setting, NoRoutes creates
no invitation, and shutdown releases owned tasks. The CLI prints the same
safe state and offers explicit enable/disable. Root/native final package gates
and fake-runtime/capture evidence remain separate from contract review.

## Costs and scope

Vessel intentionally has no hosted relay bootstrap or password prompt: pairing
redeems a single-use invitation into a device credential, rather than the
reference's reusable offer plus optional daemon password. It requires an
operator-configured relay before remote consent can enable registrations.
This work does not add a relay address editor, change TLS/Noise security,
weaken scope admission, revoke devices, or claim phone/other-OS execution.


## Isolated registration-child lifetime seam

The coordinator separately assigned this seam after source review found that
`NodeLink::open_data` detached its ten-second websocket dial/handshake tasks.
Tracking only `run()` therefore did not prove that no accept callback could
arrive after disable acknowledgement. Token checking alone leaves a
check-to-callback race.

The same NodeLink run now owns its pending data-open JoinSet, with
MAX_PENDING_DATA_OPENS =16 per registration, reserved before child spawn.
Overflow starts no dial; the relay's existing pending-client timeout remains
the refusal path. A stop cancels/aborts pending opens and joins every child
through its final accept-or-discard decision before publishing Stopped or
returning. Node Relays additionally fences accept on the same registration
token. A callback already in progress completes before that join; a carrier
already handed to the gateway has an independent owner and is not cancelled.
Transient control reconnect, Reach, TLS and ticket renewal keep their existing
paths. Dropping run cancels/aborts children but is not a normal stop ACK.
There is no second runner, transport, actor, dependency or wire change.

This isolated branch contains only that lifetime seam and its tests, on the
announced gated node base. It does not include the relay settings DTO/runtime
or CLI preparation, assign a field version, or prove the full consent flow.
Its tests and requested narrow check are pending; V3 and the final isolated
package gate remain required. The original queued370 settings check remains
predecessor source evidence and cannot cover these child dials.
