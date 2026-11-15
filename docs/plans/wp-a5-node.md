# Connected notification routing and terminal attention

Status: contract proposal, ADR 0089. No registered call or product behavior yet.

Existing assets: the node's push presence and policy, transactional inbox
projection, terminal actor and typed terminal registry are reused. Recipient
nomination is currently computed only for push suppression; terminal activity
is absent from the wire. The native app's node boundary must not guess either.

Decision: modify existing policy/registry and add a typed exact-source claim.
Rejected: a second inbox, arbitrary PTY prompt detection, client-side election,
and a new public hook listener. Standard Rust state and existing dependencies
suffice for contract values; no new external dependency is proposed here.

Next: read the reference terminal producer and existing terminal actor, specify
admission persistence/retention and the connection-bound claim outcomes, then
submit pending rows through the node integrator's Quality B. Pure types and
staged service tests precede registration. Every new wire field gets all-path
older-peer tests. No production node or real agent is used.
