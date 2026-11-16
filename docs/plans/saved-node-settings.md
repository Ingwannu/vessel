# Private saved node settings envelope

Status: implemented in the common settings producer; verification and complete
source review remain in progress. This package defines no service/plugin consent
schema or activation engine. The opaque record composes with the single accepted
operation lane and the reviewed private-cause decoder, rather than adding a
second saved-state or persistence owner.

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

## Public and private parts of one namespace

A later namespace can contain a safe generic choice and private metadata in the
same disk key. For example, the plugin namespace belongs to ADR 0079 and is added
at its reviewed merge; this proposal does not adopt its schema. The private
stored record must own that complete namespace, with an explicit typed generic
projection/merge. Naively flattening both a wire patch and another private record
with the same key would create duplicate ownership or discard private fields.
The initial P2 did not freeze that representation or promise a callback API.

The envelope therefore remains opaque outside its disk edge. New typed private
namespaces join the existing mutation owner under their owning contracts, without
a Store-to-Node dependency, an untyped placeholder or a second writer. Their
ordinary generic changes and resets must preserve all private fields; reload
must not bypass the dedicated metadata setter. Unknown/future data continues to
use the existing set-aside policy, rather than an invented partial merge.

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

Store exposes a private-field non-Copy SavedNodeSettings envelope,
without public serde/TS implementation. public_settings projects the existing
complete generic snapshot; consuming with_public_settings builds a candidate
without granting live mutation authority. The common producer's internal
Document now owns parking, drivers and the optional typed recovery patch
explicitly, preserving canonical field order and previous bytes when recovery
is absent. Store's disk methods and Node's retained mutex use the envelope;
request/reply recovery fields retain their separate unassigned protocol boundary.

The actual blocking writer returns its owned candidate with the set-aside result
so it can publish that same value after I/O without an extra document clone. The
existing validation, token/lock lifetime, set-aside/write/apply order and I/O
causes remain. Existing assertions need only typed envelope/projection adapters.
New fixtures compare current canonical bytes and semantics, prove the opaque
envelope cannot be wire-serialized, and keep a no-op's original bytes/future
fields without creating a set-aside copy. The current focused Store settings
suite and Node settings suite have executed successfully. That scoped evidence
is accompanied by the passing opaque-envelope compile-fail doctest and passive
SQL integration suite. Generated declarations and the enclosing gate remain owed.

Pending: remaining changed-source checks and whole review, later combined-base
checks, and the concrete private schema/canonical incarnation contract. No
private slot, consent setter or service/plugin activation has been implemented.
The separate config reload owner join also needs adaptation and verification
when these branches meet; it is not silently included in this P2 range.
