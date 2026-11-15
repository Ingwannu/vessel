# Canonical connection lifetime prerequisite

[Decision Log]
- Purpose: all connection-owned resources need the same unforgeable identity
  and cancellation lifetime, without retaining the actual context guard.
- Existing assets: the tested dictation owner primitive already separates an
  opaque Arc identity, observed cancellation token and shared drop guard. The
  connection's push presence id is a notification key and stays unchanged.
- Alternatives: a second key per feature permits inconsistent ownership and
  teardown; storing the guard in a resource prevents context-drop cancellation.
- Choice: reuse that pure primitive in a node module. Context setup initializes
  it once, acting-as clones share it, and resources use only the accessor. Quota
  belongs to each resource and does not become mutable identity state.
- Why: identity remains alive with resource records while the last context drop
  still cancels them. No new wire id, parser, dependency or protocol is needed.
- Costs: context drop alone can be delayed by an RPC close grace, so the
  wiring makes the observer a child of the transport's closing token too,
  without capturing the guard (see Validation).

## Validation and resume

The add-only source preserves the two existing lifetime/identity cases. The first
merged-base narrow check found a documentation-markup lint; the comment is fixed
without changing behavior. Strict node all-target Clippy and both lifetime
cases passed on the corrected merged snapshot (alongside eight codec cases).
Wired (2026-11-15, node line): `service::Connection::new` makes the life over
`Outbound::closing_token()` (a child of the transport's closing token, new in
vessel-rpc), `acting_as` clones it, and `Connection::life()` is the accessor.
The observer now fires on the transport closing as well as on the last context
drop, so a resource no longer observes `Outbound::closed` itself. A node test
serves the real guarded service over an in-memory pair: a context acting for the
connection shares its owner, and closing the transport ends the life while that
context is still held. Scope checks, resource admission and product behavior
remain with the consumers (workspace preview, W14, both speech consumers).
Until the first one reads the accessor, the non-test build expects it unused.
No OS, account, permission or running service is changed by this primitive.
