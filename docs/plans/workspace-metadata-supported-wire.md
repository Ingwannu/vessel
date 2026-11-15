# Assigned workspace metadata wire validation

Status: authored tests on assigned protocol 25; changed-head execution and review
remain owed. This supplements the registry assignment rather than enabling any
provisioning, generation, archive, config, editor or process effect.

[Decision Log]
- Purpose: a feature version pin alone cannot prove old-hop refusal, domain CAS
  or person authority on a running node.
- Existing constraints: use the real typed methods, recording RPC fake, memory
  transport and isolated node fixture. No feature override or new protocol row.
- Alternatives: treating generic transport fakes as authority proof, or deriving
  identity/backing from a cwd. Neither supplies the required fact.
- Choice: independent transport admission and real-node domain tests. Offline
  Store seeds are metadata only, closed before Node startup; same-path records
  remain independent and nonexistent backing paths are never provisioned.
- Impact: direct/self/nested old-hop SDK requests must send no registry frame;
  raw old-hop calls must fail before the recording fake dispatches, with GetNode
  as a supported ordering control. Real owner calls exercise all six methods,
  active/foreground filtering, exact identity, manual reset, pin/unpin and
  source/revision release CAS. Original accepted whole failures remain history;
  the announced assignment head has its own green combined gate.

Tests in this slice do not yet close the full assignment plan: authenticated
paired-device and real fleet forwarding/intersected-scope/Agent paths, current
nested hop and unknown-field compatibility remain explicit next cases. They
must use the existing admission/secure fixtures, never synthetic proof from
these transport fakes. No source-only test is reported as executed.
