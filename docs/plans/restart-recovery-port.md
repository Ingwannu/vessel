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
- Settings operations. A validated patch transfers to the settings owner
  before it waits for a reply. One consumer serializes a physical writer
  and up to eight waiting patches. This carries the exact outcome of a
  save through requester cancellation. It also prevents a queued On from
  overtaking any cleanup from a durably saved Off. Shutdown seals settings
  and recovery admission before it drains the lane while runtime and
  Store are still available.

## Source commits

| Commit | Selected source |
| --- | --- |
| `db372b34a1897639b217cfdd3fce2f25395a3db9` | Recovery checkpoint types, explicit Get/Set field boundary and durable Store projection. The port keeps the current method table, writer receipts and migration order. |
| `baf7ee09ab5c1d6a5705686ca5283839083174b5` | The locked startup reads one validated settings snapshot. Store bootstrap and the live settings owner use the same value and preservation decision. The port reuses the current file decoder and writer with their additional settings fields. |
| `3d19a877ecd4834eba2633a4dd420c489002239a` | Bounded FIFO transfer of accepted settings operations, including ordered application after reply loss. |
| `b2b31e674a35e57f5603eacfc9dc4b410f57e7a5`, `fe109da25fed27c1b62ac2193c2630e936a9c7cb` | Recovery policy arbiter and settlement in the retained settings writer. |
| `f655614f91d730ed1e7a7ab4eef2b18f9c0e702b` | Seal recovery effect admission with the settings lane, before runtime and Store drain. |
| `8970e4fa339fb5199e13584ca6612877c0f8efe8` | Producer regression coverage for failed file replacement, SQL cleanup and exact saved prompt preservation. |
| `37f9460f4cde60ae629779b80d60104a5fa4b225` | Ordered restart journal and shutdown capture. The port keeps runtime `LaunchAdmission`, preparation claims and the live metadata clock. |
| `12d862346c78e7bccda51baff9d4f9dda7b56913` | Node persistence consumer and test acknowledgement barrier. The port applies the committed clock before publishing the durable sequence or returning a checkpoint acknowledgement. |
| `3fc3de004e89226538ba8993e54d8c524e3dc827`, `51dc3419fcdf835c9969eba8879dd84f6e70f260` | Boot coordinator, device grant fence, ordinary-operation admission and actual provider cases as assembled at `12d862346`. Existing worktree leases, model routing and confirmed Vessel tool injection remain on the same launch. |
| `090ea2c62f5587ed76012b2eaafa0362342c5fda`, `f1c5b42fba3c3a1825ef110a66042c120a10b127`, `5970aec68e2ec1807a9f0c65960432fe74fa0a47` | Exact-run Kill boundary and its held real Arm-acknowledgement regression. The code pins the already admitted handle while recovery retains its original slot and actual prompt outcome. |
| `9d8396be1b948204d70e75bdbd1c656e88f607ac` | Client helpers, CLI settings and fake load admission. The CLI reuses the current `Switch` parser and retains the plugins patch. |

The consumer checklist is `8003b14e1f9fc26f228390ee071b15eb5b47a2fd`.
The producer reference is `385fc1ee5b0e96e1968b2025eb626a2e13e2153a`.
The exact-run Kill source evidence is `d9a9c840db88958a965091d19d75b919cbf2d908`.
The preparation view observes the current runtime's `LaunchAdmission`.
Its host callback holds the original attempt and checks live cancellation
and the deadline under the same policy arbiter as launch and prompt enqueue.
When preparation is refused, the coordinator awaits the original physical
lookup or helper before releasing its attempt and Store protection.
Failure diagnostics use outcome codes because a provider can echo input.

## Page and CLI settings

The connected host shows its recovery card after System prompt in Agents.
This follows Paseo 0.15.4 `packages/app/src/screens/settings/host-page.tsx:281–308`
and `packages/app/src/screens/settings/restart-recovery-card.tsx:19–115`.
The switch follows the node's confirmed value. The outline Edit button
opens the prompt while On, and a Cancel or an invalid input leaves the
kept value in place. The sheet uses the Vessel kit's 520 px desktop size
for Paseo's 560 px sheet (`restart-recovery-card.tsx:125–174`). A failed
update leaves the draft and error visible. A fresh read reflects any save
that took effect before the apply failure or reply loss.

The page and CLI get support through the explicit flagged read. An empty
reply after that read uses the node's Off and `Resume working.` defaults.
The CLI can change the prompt while Off because the protocol stores the
prompt and toggle independently. `vessel node settings --resume-on-startup
on|off` changes the toggle. `--resume-prompt TEXT` changes the prompt.
The node validates prompt text before accepting a settings operation.
