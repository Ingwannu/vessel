# Private plugin settings join

Status: proposed contract refinement to ADR0079. No settings field, disk slot,
writer, setter or activation engine is implemented by this document.

## Reuse and ownership

[Decision Log]
- Purpose: keep the global plugin switch, installed-source intent and registry
  configuration in one plugins namespace without exposing private data through
  generic node settings or giving a generic patch installation authority.
- Existing assets: nodectl owns the proposed SavedNodeSettings disk envelope,
  the existing settings.json atomic/set-aside behavior and the retained mutation
  guard/task tracker. W16 owns plugin schema, preparation and lifecycle.
- Alternatives: private fields in wire SetNodeSettings, a second plugin writer,
  or an opaque unvalidated JSON slot. They expose the wrong authority, split
  cancellation ownership or admit data before its meaning/bounds are checked.
- Choice: one typed private plugins slot in the existing saved envelope, plus a
  separately typed safe global-enable projection/patch. Dedicated management
  mutations prepare a complete candidate under the same owned settings guard.
- Cost: the saved model is non-Copy and generic/dedicated mutations need explicit
  preservation tests. The actual Store/Node join waits for the reviewed envelope
  owner API and announced shared head; this proposal exports no placeholder API.

The envelope extraction belongs exclusively to nodectl. W16 does not change
DataDir read/write methods, Settings.saved, or implement another persistence
queue. The saved plugins record owns the WHOLE top-level namespace, including
its global flag: it must not be flattened beside a generic record carrying
another plugins key. Disk serialization and safe wire projection/merge are
explicit, so no duplicate key or private-field loss can arise. Generic update,
reset and reload must preserve all protected fields.

The proposed data-only schema location is vessel-store::plugin_settings at the
existing Store/Node boundary, depending on canonical proto identities rather
than Node. It is not a node RPC payload and has no generated TS surface. Pure
scalar validators shared with source admission move once into that data layer;
Node retains filesystem preparation, authorization and runtime policy. No
Store-to-Node dependency or second parser is introduced. Its public cross-crate
export is wired only together with actual saved-envelope and node consumers,
not as a callerless facade. The detailed managed provenance variants wait for
their qualified acquisition producer rather than an opaque placeholder.

No current declaration inspection or parsed source spelling is a prepared
installation, executable capability or accepted saved record.

## Private schema contract

PrivatePluginSettings is immutable after validation. Its format is a local disk
V1, independent of the pending public PLUGINS protocol and SQLite numbering.
It contains the global saved enable intent, a configuration revision, distinct
installation records and configured registry records. Missing plugins means Off;
missing installation enable intent follows Paseo's true directory default while
still remaining subject to the independent global Off floor.

The maximum installed record count reuses MAX_CATALOG_INSTALLATIONS (128), so
inactive entries cannot multiply management state beyond the node's public
catalog capacity. Additional named private limits are 32 registry records,
4096 UTF-8 bytes per source/path/origin, 16KiB per authorization value, 64KiB
aggregate authorization bytes, and 1MiB complete canonical private slot. The
decoder checks bytes before parsing at its named file entry. Typed construction
uses the existing counting sink pattern; generic serde is not a universal
pre-allocation boundary. Duplicate keys/ids refuse before map insertion, rather
than letting the last authorization/installation silently win. Errors seal raw
parser details and every Debug/Display/source level.

Installation records use canonical PluginId and preserve individual enable
intent independently of global state. Directory records contain private resolved
node-local source identity; they never authorize deletion of that directory.
Managed records identify an owner-created managed generation and qualified
Git/npm/registry artifact provenance from the actual acquisition adapter. Neither
an arbitrary path nor DirectoryInput/GitInput nor the declaration-only hash pair
may construct that qualified record. Full artifact identity, source containment,
owner/mode validation and candidate review remain installer prerequisites.
Current helper RuntimeGeneration, PID, ProcessLifetime, catalog tickets and
in-flight operations are never persisted as installation or cleanup authority.

Registry records use the reviewed URL/origin validation, explicit native registry
selection and closed validated identifiers. Authorization is a private secret
value with fixed redacted Debug/Display, no raw-text error causes and real header
syntax validation. It is available only to the node fetcher for its exact
configured scheme/host/port origin. Redirects refuse; neither helper argv/env nor
Read replies, public catalogs, scoped errors or ambient traces receive it. The
existing stronger HTTPS-or-loopback policy and absent public native default are
unchanged. This slot does not make an actual fetcher available.

A safe PluginGlobalSettings projection contains only saved global intent and
its optimistic configuration revision. It has no path, source, registry origin,
authorization, diagnostics, installation mutation or broker grant. The generic
patch can change only that safe global intent. Its protocol gate remains None
until the integrator assigns PLUGINS; old/unassigned peers cannot mutate or see
new fields through nested/forwarded settings paths.

## Mutation admission and effects

The existing settings owner acquires its owned saved guard, checks the complete
candidate and no-op/revision predicates, then transfers that same guard into its
tracked persistence/application task. Caller cancellation drops a waiter, not
the guard, write or runtime application. Shutdown closes admission and joins it.
No separately acquired plugin lock becomes another writer or a deadlock cycle.

The accepted retained-writer repair currently permits a late park/driver apply
after session runtime shutdown because those setters cannot spawn a process.
That result does not qualify plugin start callbacks. The existing node teardown
order stays owned by nodectl; W16 does not reorder it independently. PluginHost
uses the node Lifecycle stopping token and its own serialized admission/owned
child registration fence. A late persisted enable after that fence closes must
never start a helper or republish a live catalog. Existing Supervisor refusal
for an already-cancelled parent is necessary, but is not by itself proof against
a check/start race or complete host shutdown.

The application result distinguishes saved desired intent from current effective
runtime state. A writer that completed after stop may retain saved On for a
later validated startup, while the current effective result is Stopping or
NotApplied; it cannot acknowledge Running. Disable/remove still require owned
cleanup evidence before acknowledging their lifecycle effect. The actual join
must exercise held file I/O plus caller cancellation plus node shutdown, late
enable refusal, admitted-start cleanup and disable acknowledgement. Both the
settings writer and plugin owner remain tracked until their admitted work joins
before Store close. AA's scoped pass is not borrowed as plugin runtime proof.

Installation, source/registry edits, update apply, reload, disable and remove
have dedicated human local-owner/paired Admin admission. The safe global toggle
also requires human Admin admission, including when carried by the generic
settings patch: its safe shape does not grant permission to activate code. An
agent/delegated or plugin-broker principal cannot reach either route.
Prepared mutations privately bind actual node owner, current configuration and
installation revision, candidate identity and exact executable target. A stale
proposal refuses before effects; a client assertion that it reviewed code is
not a replacement for the node's captured candidate. Every changed file/target
must pass the owning acquisition/inspection checks again before use.

Generic writes preserve all private installation/registry fields. Generic reset,
key edits and explicit reload are not private-plugin setters. A reload that
changes protected plugin metadata refuses before publication; it cannot start
new code or silently discard an installation/credential. Future/unrecognized
private data never authorizes activation. Its exact preservation/downgrade
behavior must join the envelope owner's reviewed unknown/future-file rules;
this proposal does not invent an opaque payload or automatic migration.

A saved global enable value is intent, not proof that a helper is running.
Startup revalidates configured packages and settles each helper into Running or
a typed Failed state before the authoritative initial catalog. Failed start may
retain saved On intent for an explicit retry. Turning Off first prevents new
admission and withdraws contributions/revokes broker use during runtime apply,
then cancels and joins owned helper cleanup. Existing write-before-apply ordering
means a failed persistence operation does not revoke the old runtime early. The result distinguishes persisted intent from failed or
uncertain cleanup; wait_checked proves leader reap plus accepted first cleanup
request, not all-descendant death. No management acknowledgement may claim the
latter from a saved flag or ProcessLifetime revocation.

## Required join evidence

- Safe generic read/patch/forwarded downgrade never contains private fields and
  cannot mutate an installation, registry authorization or broker policy.
- Direct human Admin admission and exact candidate/revision/target checks happen
  before filesystem, source process, network or executable effects.
- Actual concurrent generic and dedicated writes preserve each other's fields
  through the one writer; cancelled callers and shutdown retain complete I/O and
  publication ownership. Failed writes preserve previous active intent.
- Duplicate ids/keys, malformed private headers, future format, count/byte caps
  and full-chain diagnostic canaries exercise the real decoder/file boundary.
- Actual owner-only file modes and generic no-op/future-copy/set-aside semantics
  remain those of the existing settings owner. No new metadata file is created.
- Global Off blocks new work; removal never deletes user directories. Runtime
  cleanup uncertainty remains explicit and cannot be flattened to success.

Until that concrete private schema, producer, saved-envelope join and lifecycle
pass review and actual execution, plugin settings/installation remain unavailable.
