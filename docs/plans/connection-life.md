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
- Costs: context drop alone can be delayed by an RPC close grace. Wiring must
  also observe Outbound::closed, without capturing the guard in that observer.

## Validation and resume

The add-only source preserves the two existing lifetime/identity cases. The first
merged-base narrow check found a documentation-markup lint; the comment is fixed
without changing behavior. Strict node all-target Clippy and both lifetime
cases passed on the corrected merged snapshot (alongside eight codec cases).
It is staged until the connection setup wires its field and accessor; this source does not prove actual transport close, scope checks,
resource admission or product behavior. Next: initialize in one context setup,
clone on acting-as, then both speech consumers use that same identity/observer.
No OS, account, permission or running service is changed by this primitive.
