# Creation count and launch correlation dependency

[Decision Log]
- Purpose: let the one creation owner retain the existing Agents count lease
  and correlate its post-Begin concrete launch capsule with the same entry.
- Existing constraint: legacy reserve returned a credential and replaced duplicate
  ids after logging. A durable retry must never overwrite that entry or remint
  its credentials. A reply future cannot own rollback for a tracked launch.
- Alternatives: a second counter; a free-standing permission stamp; or a retained
  guard over the original registry entry with private identity correlation.
- Choice and reason: the existing Agents reserve delegates to AgentReservation;
  duplicate ids refuse before minting. Legacy success explicitly keeps the same
  existing lifecycle. A staged count correlation uses that registry reference,
  child id, private Arc marker and original Begin operation/revision.
- Impact: creation-specific Begin/refusal/correlation and entry phase state remain
  literal cfg(test). The current-parent fence and actual tracked concrete capsule
  are separate producers. No correlation authorizes any launch by itself.

The canonical creation value bodies accompany this dependency because the count
relation uses their exact operation revision and pure receipt transition types.
Only the module registration and these values are included. Observation request
and method registration are deliberately absent: there is no RPC, Store/schema,
profile setting, filesystem provisioning, runtime launch, or protocol assignment.
The value constants describe receipt budgets; this carrier does not enforce a
persistent ledger. No second revision/id type or copied registry is introduced.

Reserved, Begun and Admitted are states inside the existing count entry. A known
pre-admission refusal clears the old Begin binding before rollback. An admitted
launch cannot be undone as a known no-spawn refusal. The common Agents guard
must check the current parent and insert into the concrete owner capsule in the
same critical section; the helper has no callback, I/O or internal registry lock.

This is a staged dependency for the joint execution-fence review. Named tests
exercise duplicate/capacity/rollback and exact registry/marker/Begin correlation,
including same-id replacement, current connections and consumed admission. Fresh
receiving checks are required. It supplies neither parent execution authority nor
any production creation API, and is not a whole A1 completion claim.
