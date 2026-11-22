# Startup recovery on the node line

F-L24 implements the settings controls and startup behavior from ADR 0059.
The port starts from `1829d53dc` at protocol 48. The coordinator selected
`backend/m12-recovery-hook` at `12d862346` for a selective port.

## Decision log

- Purpose. Resume a committed interrupted agent turn on the owning node
  with the configured prompt and the current device grant. A person's
  stop takes precedence over startup recovery.
- Existing owners. Settings owns the saved file, serialization and
  publication. Store owns durable event ordering and the recovery
  projection. SessionRuntime owns the actual `LaunchAdmission`, provider
  preparation and process spawn.
- Alternatives. Importing the staged branches would also bring older
  settings, schemas and launch adapters. A second settings writer or
  launch fence would create two owners for the same operation.
- Choice. Extend the current owners with the staged policy settlement,
  ordered journal and recovery consumer. Qualify preparation through the
  actual launch admission. Use the original attempt through physical
  settlement and pin a person's Kill to the exact admitted run.
- Effect. A saved Off consumes pending intent before a later On can
  proceed. A failed Off write restores the original admission. Recovery
  metadata adds one schema migration. The named protocol remains
  pending until integration binds its client surfaces.
- Store. The recovery projection shares the existing writer savepoint
  with create, reopen and ordered checkpoint writes. The caller receives
  the same committed mutation clock as on an ordinary write. This keeps
  live metadata and attention wakes consistent even when the checkpoint
  itself is stale. Existing rows start without recovery intent because a
  historical start event cannot establish a committed recovery checkpoint.

## Source commits

| Commit | Selected source |
| --- | --- |
| `db372b34a1897639b217cfdd3fce2f25395a3db9` | Recovery checkpoint types, explicit Get/Set field boundary and durable Store projection. The port keeps the current method table, writer receipts and migration order. |
| `baf7ee09a` | The locked startup reads one validated settings snapshot. Store bootstrap and the live settings owner use the same value and preservation decision. The port reuses the current file decoder and writer with their additional settings fields. |

The consumer checklist is `8003b14e1f9fc26f228390ee071b15eb5b47a2fd`.
The producer reference is `385fc1ee5`, and the exact-run Kill reference
is `d9a9c840d`. The implementation records the selected commits below as
each portion is integrated.
