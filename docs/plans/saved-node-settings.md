# Private saved node settings envelope

Status: proposed. This package does not define a service consent schema or a
service activation engine. The shared tracked settings writer must be on the
accepted node base before implementation. No implementation or runtime evidence
is claimed by this plan.

## Existing assets and choice

`[modify]`: Store's `settings_file` owns the one `settings.json` path, atomic
replacement, owner-only file permissions, unreadable-file set-aside and the
copy-before-write rule for fields a downgraded build cannot retain. Node's
`Settings` owns saved state, launch-value precedence, validation and publication.
The prerequisite retained mutation repair keeps its guard/tracker through I/O
and apply despite caller drop, and shutdown joins admitted mutations. It must be
on the accepted base before implementation. These owners and the file remain
the same.

The current disk API uses `SetNodeSettings` as its model. A generic wire patch
must not grow authority over private service consent merely because that consent
shares the file. Introduce a private-field `SavedNodeSettings` envelope at the
Store/Node boundary. Its generic projection and replacement reuse the current
typed settings fields; they do not create another CLI key catalogue. Disk
serialization remains internal to Store, while Read replies continue to use
`NodeSettings`, never the envelope.

The first change is behavior-preserving: preserve current canonical writer bytes
and existing missing/default/null/unknown-field behavior, future-data detection,
set-aside rules, no-op behavior and write-before-apply ordering. The model change
must not secretly adopt strict reload parsing for ordinary startup or mutate a
file to migrate it. No new file, wire call, dependency edge or numeric bound is
needed for that separation.

## Alternatives and limits

A second services file/writer would duplicate serialization and cancellation
ownership and is excluded. Adding service consent to generic `SetNodeSettings`
or its generic key map would admit the wrong mutation surface. An opaque JSON
placeholder is also excluded: it would create an authority-bearing payload before
its canonical producer types and resource bounds have been reviewed. Internal
reuse of the typed generic patch avoids copying its field definitions, but its
disk adapter must prove byte parity rather than assume serde flattening is free.

Paseo's `packages/server/src/server/daemon-config-store.ts:393-406` queues supported
patches, `484-485` queues reload and `655-659` serializes both. This supports reuse
of one mutation owner; it does not define Vessel's private service-consent data.
Vessel's private envelope is an explicit authority separation, not a claimed
Paseo wire or file format.

## Protected services follow-up

`services` is reserved for a separately reviewed private schema and a dedicated
Admin setter using the same settings owner. Generic settings mutations must not
reach it, generic writes must preserve it, and Read replies must never expose it.
An explicit reload must not become a second service setter. The contract for
handling a private-slot difference in an externally changed file must be settled
before wiring; no silent discard or implicit activation is permitted.

The service key is the canonical `WorkspaceId` and `scripts::ScriptName`, with no
cwd/project/session inference. Consent binds to `workspace::CommandsDigest` and
the qualified physical workspace incarnation. Current-process lifetime, PID and
per-run generation are never persisted as consent. A fresh run still needs its
own socket/run validation before a public route can activate. Until the owning
incarnation and private record/count/byte contracts are qualified, activation
remains unavailable and this package invents no substitute identifier or payload.

## Owed tests and handoff

- Compare envelope writer bytes with the current canonical writer across today's
  complete supported saved settings forms, including absent fields.
- Keep the existing unknown/null/default/set-aside/future-copy assertions; add
  adapters only where the opaque API changes a test's type, never weaken its
  behavioral expectation. A no-op must preserve the original file bytes.
- Exercise the same retained mutation owner after its saved type changes, with
  actual I/O cancellation, later mutation ordering and shutdown joins.
- The later protected-slot package must prove generic/dedicated concurrent
  mutation preservation, reserved-key admission, direct Admin authorization,
  no Read/diagnostic disclosure and no persisted current-run authority.

## Resume point

Done: read current Store/Node/Paseo ownership and record the separation contract.
Pending: accepted node base with the tracked writer, concrete private service
schema/canonical incarnation contract, source review and exact changed-source
checks. No disk-model code, private slot or setter has been implemented here.
