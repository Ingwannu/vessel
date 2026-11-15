# Creation operations and terminal profiles

ADR 0091 records reference behavior, authority, alternatives and compatibility.
ADR 0082 owns workspace identity/provisioning; consume its reviewed contract
instead of deriving identity from cwd or persisting another registry.

## Structure

- `vessel-proto::creation`: validated keys, revisions, phases and typed receipt
  snapshots; create/observe contracts composed with the shared registry types.
- `vessel-proto::terminal_profiles`: bounded profile values and pure literal
  prompt substitution; no process or configuration I/O.
- `vessel-store::creation`: admission/resource claims and receipt transitions
  through the existing SQLite writer/read pool, migration appended over head.
  Reserve count and worst-case retained logical bytes atomically before
  effects; retain every key/claim indefinitely, with typed capacity refusal
  instead of eviction. Existing-key reconciliation and bounded phase growth
  remain possible at capacity. ADR 0091 names the limits and required tests.
- `vessel-node::creation` and `service/creation`: admission, operation actor,
  revision observation and delegation to workspace/session/terminal services.
- Existing terminal/config services: persist profiles with existing protected
  settings writes; launch using existing process/group/environment handling.
- Existing method table: pending rows only after Quality B; no alternate
  dispatch or provisional version number.

## Resume point

Done: profile values at 5c39da5f passed six narrow tests and strict proto
Clippy. Native pure-state tests passed five cases. Shared registry types were
consumed for development only with their owner's approval; source is not yet
reviewed. Node head Quality B was merged after the first receipt/store WIP.

Review row 125 returned the proposed receipt capacity contract, with no profile
source finding. The contract now defines atomic retained count/byte admission,
worst-case growth reservation, permanent key/claim retention and refusal at
capacity. Receipt/store code remains WIP and is not submitted as compliant:
the initial count-only implementation still needs the byte reservation and
adversarial tests before any service/RPC/launch integration.

Next: capacity contract re-review; implement byte reservations and receipt
tests; narrow checks; receipt source review; then service orchestration,
pending-method registration, native integration, package gates and reviews.
