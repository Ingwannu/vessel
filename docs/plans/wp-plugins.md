# W16: Plugins and plugin-contributed themes

ADR: 0079 (architecture CONTRACT PASS by V3; implementation unreviewed).
Scope: F6/W16 in PLAN.md section 7, which supersedes duplicate assignments.

Current node source is the clean export on ede20014 plus portable checked-wait
b37a3ee8, with no W15 release/desktop ancestry. Its helper/manifest/catalog
models are staged, not activated by node methods. Earlier checkpoints below
name their historical bases and exact evidence; those passes do not prove this
current combined source. The app remains on its separately merged shared line.
F6 is still skeleton, with installer/management/public RPC/Hub/rendering and
whole-package evidence outstanding.
Reference source is Paseo 0.15.4. ADR 0079's source table and differences
are the contract for this package; do not duplicate its protocol list.

## Asset and reuse decision

Modify and reuse native view/link/app layering, shared kit controls,
vessel-proc, RawJson, scoped authorization and accepted ADR 0068 machinery.
Add plugin-specific declaration/registry types: automation hooks are HTTP
deliveries, core routes are closed and provider options are not extension
identities. No external UI/runtime package solves React on GPUI. Choose
new declaration models with existing controls, not a new UI runtime.

## Ownership and data flow

Node installation -> supervised helper -> validated registration/catalog ->
link's per-node catalog -> view's registry -> app adapters -> kit controls.
User action -> validated target/generation -> link -> authorized node ->
helper. Theme selection is local; plugin installation/config is node-owned.

Native `view::plugins` owns validated identity, declarative contributions,
host catalog replacement, theme validation and selection. `link::plugins`
will own only available typed node methods and cancellation/reconnect;
`app::areas::plugins` will draw those states. Existing feature owners receive
contributions through narrow adapters, never through shared-shell logic.
The accepted hook track has dirty, uncommitted code and is not a dependency
until its owner provides a reviewed hash. Do not copy those files.

## Implementation order and verification

1. Commit ADR and plan; fast native fmt/size/boundary/token checks; submit
   ADR to the designated node reviewer before code.
2. Skeleton: typed pure contributions/registry/page contracts, hidden area
   entry and link unsupported boundary; narrow clippy and changed tests.
3. Fill native models and tests against malformed input, duplicate identity,
   stale generation, host collision, offline/no-fallback, removal and theme
   parsing. Refuse unknown or unavailable contributions explicitly.
4. Node runtime, sources/settings/hooks and CLI require authorized shared
   protocol/owner integrations. No fabricated RPC or product fake.
5. Only after actual user-visible wiring: fake fleet captures, inspected
   EN/KO dark/light 1280/390, whole native gate once. Root changes also get
   the root gate. One app package review with verified/blocked/unverified
   explicitly separated; security commits retain their required review.

## Current boundary

The initial baseline 1d1f11df had no plugin RPC. The app tree now merged
package base 4, 3cd0d3fc, under A8, including Quality B and the reviewed auth
race correction c5da5305. The node tree has Quality B 29eebb8b. No feature is advertised
as complete until its real path works. The package is not a whole-package
gate or runtime/capture proof merely because its declaration tests pass.

Coordinator subsequently authorized W16 to implement both sides in a
second node worktree based on d954fa3a. The integrator confirmed one pending
PLUGINS Option None and one settings namespace; no SQLite schema is needed
for owner-private settings/source files. Quality B now permits pending rows
in the one method table; the node service must exist before those calls can
be meaningfully consumed. Workspace identity belongs to W11 ADR 0082, pane/tab
identity to W12, chooser/fallback/preferences and token adapter to K, and
command-center registration to W22b. Consume their reviewed contracts;
do not infer workspace identity from a path or allocate another WorkspaceId.

Staged scalar/address/theme values are in `vessel_proto::plugins`, with no
dependency or manifest change. RuntimeGeneration is a public UUID correlation
value, not an authorization credential; plugin and contribution addresses
retain NodeId and generation. HexColor parses once and retains packed RGBA;
no fallback color is synthesized after failed parsing. Whole documents,
helper, node service, CLI and native adapters remain separate implementation
work and are not claimed by the scalar tests.

## Native skeleton and request ownership

The app tree merged package base 4, 3cd0d3fc, without conflicts; node work
merged Quality B 29eebb8b. Shared registration follows the split command,
event, dispatch and shell/events modules from that base. No size record
has been raised.

`view::plugins::Registry` retains validated themes per NodeId and exact
PluginAddress. Offline retains display data, never authorizes a remote
action; Unsupported withdraws that host. A complete replacement validates
all owners and duplicates before publication, while a malformed replacement
retains the previous snapshot plus a typed failure. Read tickets reuse the
existing checked ticket allocator and add a registry lifetime fence, so a
new registry's reset counter cannot admit an old callback.

`link::plugins` is the typed CatalogSource boundary, tested with in-memory
and never-answering sources. Its current product adapter returns Unsupported
without sending a method; it is not a product fake. Catalog requests have
one named finite deadline and return their captured ticket on every result.
The feature is not wired into the app's visible Settings route yet. Its
actual runtime, source installation, catalog cache, i18n, declaration paint,
per-feature adapters and capture matrix remain required work.
The coordinator authorized the exact W11 identity-only commit e45a5d89
after V3 source review. It is merged on the node tree; no unreviewed
workspace registry or store implementation is adopted. Header buttons and
composer pills carry that WorkspaceId. Actual ownership resolution remains
behind the workspace owner's typed contract.

## Declarative graph boundary

`ClientDocument` owns complete validation before publication: scoped
contribution identities, sidebar header/footer namespaces, distinct slash
names, registered method/screen references, control ids, choices, panel
locations, literal file selectors, menu/popover recursion and whole-document
budgets. A counting sink enforces the encoded cap even for typed in-process
construction without allocating another encoded document. Unknown fields,
unknown control kinds and oversized graphs refuse the whole replacement.

Paseo references read for these details: `packages/app/src/plugins/evaluate.ts`
89-92, 304-346, 391-413; `packages/plugin/src/client/buttons.ts` 6-55;
`packages/app/src/plugins/sidebar-groups.ts` 1-110; and
`packages/app/src/file-panels/resolve.ts` 45-90. A stored attachment filename
is not reused as a selector: Unix dotfile/reserved-name matchers are literal
patterns, not names the node writes. Extensions allow Paseo's literal Unicode
and punctuation except dots, separators and wildcards; matching lowercases
Unicode while exact filename matching remains case-sensitive. Native bounds
and refusal of control characters are additional resource/display constraints.

Slash names remain the shared `conversation::CommandName` for the composer;
document validation also enforces Paseo's stricter plugin registration grammar
and uniqueness without reinterpreting a noncanonical command key. B2 owns built-in/plugin/provider precedence and trims submitted
arguments once. Sidebar grouping/order/preferences belong to S2; header and
footer may reuse a local id and therefore have separate address kinds.

`ContextAttachment` captures a validated attachment-source address. Its logical
node/plugin/source/item key survives a helper restart for draft selection;
that key never authorizes another search/invoke. Search replies refuse duplicate
or excessive items. Context URLs use the already-owned URL parser with input-free causes and
HTTP(S)/explicit-authority/no-userinfo policies behind an additional item-byte cap, an explicit difference from Paseo's
general URL schema. No URL is fetched or opened when registering context.

`LogText` redacts Debug/Display and serializes its bounded text only for the
future management logs call. The log count/byte/line limits follow Paseo
`plugins/runtime.ts` 41-46 and 1378-1398; invalid native stderr bytes are
decoded lossily before enforcing the final UTF-8 cap and truncation is explicit.
No ambient trace should contain helper diagnostic text.

## Plugin localization contract

L confirmed the core Translator is deliberately generated/embedded and has no
dynamic-key loader. Plugin resources stay in their own installation namespace,
using typed literal/message references and bounded Fluent resources with English
mandatory, canonical locale tags, English key/argument contract validation and
per-message English fallback. Reuse existing Fluent dependencies, prepare on
catalog/language events, return retained validated captions/names, and retain
raw references for language changes. Disposal/generation fences prevent stale
labels republishing. No locale RPC and no edits to core translator policy.
The existing scalar names are literals; the dynamic resource implementation is
still required and is not claimed by the current schema tests.

## Decoder diagnostics

Self-audit found that a fixed DocumentError message still exposed serde's
unknown-field/variant input through Debug and Error::source. PluginJsonError
retains the parser's concrete cause but seals that raw diagnostic chain;
only category and line/column leave the edge. Canary fixtures traverse every
exposed cause. The generic ClientDocument serde implementation also seals
structural decoder errors, so a containing envelope cannot repeat rejected
field/variant/value text. The named byte decoder retains the original concrete
sealed parser cause; both paths validate the complete graph. File selectors
reuse JavaScript edge trim for equality, rejecting rather than normalizing
BOM-wrapped input. No executable helper or native renderer is implied by these
graph tests.

## Private helper value contract

`plugins::{NodeToHelper,HelperToNode}` is the first local format slice, not a
parallel authenticated RPC API. The node assigns PluginAddress and monotonic
nonzero InvocationId; version1 and 4MiB frame/3MiB opaque-payload budgets are
named once in proto. HelperPayload seals Debug; LogText holds private callback
diagnostics separately from safe HelperFailure classes. Fourteen HookName
values share HookWait with the future core adapter; subscription duplicates
refuse. The transport will reject unsupported families instead of claiming
partly implemented provider/tool/usage integrations. No process is spawned and
no product caller, node method, protocol number or settings key is registered
by this DTO commit. Transport/supervision drafts remain scratch until their
ownership, limits, cancellation and non-root fake-child proof are complete.

## Staged helper supervision

The crate vessel-plugin now owns bounded framing, one correlated callback actor,
private logs and an owned process supervisor. The node still has no install,
management or callback RPC caller. Admission is16 calls and8MiB across queued
and active work; two-frame pipe queues and30s callback/initialization/write
budgets are named in runtime_types. Cancellation retains count/byte permits
until actual settlement or permanent retirement. Full cancellation queues retire
the helper. Monotonic correlations classify retired versus never-issued replies
without an unbounded tombstone set; generation changes revoke pending replies.

A self-audit found that a ready reply and expired timer could be arbitrated in
the same select turn. Settlement now checks the captured deadline before
removing pending admission; its regression sets that private deadline directly,
without a timing race or sleep. The actor then retires with the timeout class.

Shutdown now consumes portable checked proc b37a3ee8 and returns its actual
OwnedGroupExit. The earlier622 checkpoint is historical only and its desktop
ancestry is not part of this clean package.
That receipt proves leader reap plus accepted first group cleanup request,
not descendant OS death. Cleanup refusal remains a concrete typed error and
must prevent node replacement/disable acknowledgement. No such node lifecycle
caller is activated yet. Linux fake-helper ownership/descendant fixtures are
source only until executed on the build VM; no macOS/Windows, real permission
refusal, installed runtime or unsandboxed isolation proof is claimed.

Native package admission is staged in node::plugins::manifest under cfg(test).
It reuses canonical IDs/text/RelPath/URL and sealed parser errors; no new
external dependency or filesystem/network/process effect. ADR0079 records the
native format/protocol-floor compatibility difference, exact-target policy
and named resource budgets. Pure validation is not source preparation,
installation, runtime selection or product lifecycle acceptance.

Private stderr now drains to EOF after checked leader cleanup while protocol
reader/writer tasks are explicitly cancelled. A named two-second drain budget
refuses a pipe retained by surviving/escaped code instead of hanging shutdown;
expiration is a typed failure, not a successful complete-log claim. The real
fake helper emits its long/truncated line and final diagnostic only on shutdown,
so the retained-tail fixture covers this ordering instead of relying on startup
output. The actor still permanently withdraws all live capabilities first.

The canonical ClientCatalog/ClientInstallation value boundary now admits a
complete owning-node snapshot, exact generation, required client protocol and
immutable documents only. No private source path, registry credential, raw log
or preparation argv enters it. Named proto bounds are128 installations,
256 aggregate themes per node and4MiB complete canonical encoded catalog,
additional to individual document limits. A cached encoded size permits later
client-wide admission without formatting/serializing during paint. The existing
counting sink is factored once for graph and catalog typed construction; no
second oversized serialization is allocated. Nested generic serde errors are
sealed, and invalid owner/duplicate/zero-floor snapshots refuse atomically.
No public method or app network call is added by this value-only checkpoint.

Parent hygiene: the runtime also needs no W15 release/detach API. Its public
synchronous spawn checks for a live Tokio handle before invoking ordinary
shared-head proc, so the no-reactor path is a typed failure instead of relying
on W15's unreviewed SpawnError::Runtime addition. A synchronous regression
covers refusal before any helper is launched. A clean source export will use
the portable checked-wait prerequisite and omit W15 ancestry entirely.


Clean export hygiene: all owned plugin/proto/manifest source blobs through
b5e1830b6 match prior2ecf5ef7c exactly. The only carried process prerequisite is
portableb37 onede20014. No local-node crate, local_launch DTO, desktop store/bin
support or Released/Releasable proc state is introduced. Public supervisor spawn
has its own checked live-reactor admission, so it needs none of those APIs.

## Source reference spelling stage

Node::plugins::source is test-only source-input admission, not acquisition.
Paseo protocol/plugin-source-reference.ts and its fixture table, plugin-registry.ts,
managed-source/npm.ts and managed-source.ts438-490 define the observed precedence:
explicit directory shape; original host:port registry reference; registry slug;
explicit npm or bare npm package; otherwise unresolved explicit Git input.
Colon package subpaths must not consume URI ports, SCP's separator or Windows
roots. Native uses canonical RelPath for portable subpaths (including dot-root),
so control/metadata-directory/wildcard filesystem semantics are not invented.
Bracketed SCP IPv6 separators are handled as one host, correcting the reference
lexer's first-colon corner rather than claiming it worked there.

These are input types, deliberately named DirectoryInput/GitInput. No stat,
canonical containment, absolute-node path, Git URL/SSH/auth/ref validation,
revision/artifact pin, build or process capability is minted. Those adapters
remain unsupported. Npm selector grammar is bounded literal data; npm owns
semver/range/tag resolution. Resource bounds are source4096 bytes, package214,
selector256; no new regex or hand-written semver dependency. Diagnostics and
Debug seal private source/path/credential-bearing text.

Registry origins use the existing URL library and reject userinfo (including
empty @), query/fragment, malformed authority and non-HTTPS except standard
loopback HTTP. This is stricter than Paseo's unauthenticated public HTTP registry
rule, to keep native artifact provenance transport secure. No fetch, redirect,
credential lookup/header or child environment is performed at this stage.
There is intentionally no fabricated/public default native registry: a bare
owner/slug requires explicit native-compatible registry configuration, while a
host/owner/slug reference can identify its own HTTPS origin. Paseo's published
TS/React packages are not silently treated as native author-port artifacts.

The real-helper cancellation fixture now waits for the helper's actual request
receipt before cancellation. Previously it could cancel before actor admission,
so it was only a pre-cancel/owner-drop case. Private log retention now publishes
an optional typed sequence via bounded watch (no diagnostic text in that event);
management snapshots remain separately authorized. The fixture waits on that
publication instead of filesystem polling/sleeps, then cancels an actually
received request and verifies owner-drop cleanup. This adds no public node log
method, broker grant or new private protocol message family.

## Declaration file inspection

[Decision Log]
- Purpose: connect native manifest/document validation to actual package files
  without treating a pathname or a parsed graph as installation authority.
- Existing asset: workspace Root confines relative paths; files::read opens
  regular files, reads bounded pages on the blocking pool and reports a hash.
- Choice: reuse those mechanisms, then require a complete UTF-8 page whose size
  and SHA-256 agree with the bytes decoded. Manifest and document retain their
  existing64KiB/1MiB bounds. No new filesystem abstraction or dependency.
- Reason: a new plain read would duplicate path policy and could wait on a FIFO;
  metadata/time-only revalidation would miss same-size edits.
- Tradeoff: the workspace same-account path/open race is unchanged. Inspection
  binds only the two declaration files; it does not pin helper/build/assets or
  prove private staging, account ownership, source revision or executable trust.

Inspection is still a private test-stage module, without a node method. It reads
vessel-plugin.json, checks both protocol floors, and optionally decodes its whole
client document. Recheck re-reads both files and compares exact content hashes;
even equivalent JSON with different bytes invalidates the prior inspection.
It never runs build commands or helpers, fetches media or mutates source files.
The concrete workspace FileError is retained behind fixed safe diagnostics and
a closed source chain so missing private paths cannot leak through reporters.
Tests use isolated temporary files, including escaping symlinks, directories,
malformed/binary/oversized data, changed bytes and diagnostic canaries. These
new filesystem cases have not yet executed; pending earlier narrow jobs do not
cover them. Full artifact pinning and a cancellation-safe installer remain
required before activation.
