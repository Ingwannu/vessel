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
- `vessel-node::creation` and `service/creation`: admission, operation actor,
  revision observation and delegation to workspace/session/terminal services.
- Existing terminal/config services: persist profiles with existing protected
  settings writes; launch using existing process/group/environment handling.
- Existing method table: pending rows only after Quality B; no alternate
  dispatch or provisional version number.

## Resume point

Done: source inspected, independent node tree on d954fa3a, ADR written. Native
pure-state skeleton in its separate tree. No node product changes/gates yet.
Next: terminal profile values/tests; consume reviewed registry identities;
receipt values/store tests; service orchestration; pending-method registration;
native consumer integration; package gates and node/native review.
