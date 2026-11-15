# W16: Plugins and plugin-contributed themes

ADR: 0079 (architecture CONTRACT PASS by V3; implementation unreviewed).
Scope: F6/W16 in PLAN.md section 7, which supersedes duplicate assignments.
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

The initial baseline 1d1f11df had no plugin RPC. The app tree now includes
announced package base 2b806402 under A8, including shared field identity,
menu input leases and the reviewed arbitrary-content popover seam. The earlier dd13ce37 lock correction is retained.
The separate node tree includes announced 82ae7c91 and portable b37 cleanup,
without W15's desktop/release ancestry. No feature is advertised
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

## Retained native form and action state

[Decision Log]
- Purpose: native controls need retained edits and captured intentions without
  granting callbacks merely because an old element is still on screen.
- Existing asset: canonical ClientDocument/controls and exact-host Registry;
  the renderer will use kit fields, choices, switches, buttons and menus.
- Choice: SurfaceState privately captures one checked contribution surface and
  stores typed Text/Choice/Toggle values. Every submit/activation re-resolves
  the exact address and graph. A queued SurfaceRequest must be revalidated
  before dispatch; node caller/context authorization is still separate.
- Reason: labels and menu indices cannot identify controls, and an offline
  retained declaration is display data rather than invocation authority.
- Tradeoff: native author-port forms emit typed client intentions; their future
  node wire adapter remains unimplemented. No new method or helper call exists.

Controls inherit hidden/disabled menu and popover ancestry; traversing directly
to a nested field cannot bypass it. Menu paths use declared identities and
separators never dispatch. Choice edits refuse unknown options or value-kind
mismatches; empty literal text is retained without command/JSON interpretation.
All fields together share MAX_SURFACE_EDIT_BYTES, derived from the canonical
document byte budget, so many individually bounded edits cannot multiply
retention unchecked. A refused edit leaves the old complete draft unchanged.
Private text/static payloads are excluded from intention diagnostics.
Offline edits remain local, while submission and queued intentions refuse;
generation replacement never falls back to another node with the same plugin.
Tests cover these transitions against the existing in-memory registry. Actual
GPUI rendering, async result correlation, node transport and UI error words are
still required and no current test proves those paths.

V4 row230 found that checking only the receiving state's current graph and
address did not bind an older request to its originating graph. A catalog could
withdraw an entry, reuse the address for graph B, and let state B admit request
A. Requests now retain an immutable Arc of the originating complete document;
revalidation compares that origin with the receiving state and its live graph.
The registry shares the existing checked allocation, with a pointer fast path
for ordinary current reads; no whole document is copied into each request.
V4 row232 source-reviewed the correction; changed-source execution remains
required. Regressions transfer request A into newly constructed state B after an
empty catalog and control removal, disabled ancestry or method replacement.
Equivalent unchanged graph reconstruction remains valid and retains the captured
literal even after later edits. Exact30bc now has30 passed/0 failed/0 ignored,
including both origin regressions. Its strict check found an unused withdrawal
receipt in the test; the097 correction asserts Applied rather than discarding
it. Current strict and actual App/Kit renderer execution remain separate debt.

## Staged GPUI contribution renderer

The app's plugins module is deliberately cfg(test) until the real node-backed
Hub and canonical panel registration are wired. Its surface entity borrows the
one injected registry through a weak owner reference; it does not start another
catalog loader or clone a registry. Losing that owner immediately refuses new
intentions. Real kit TextField entities retain focus and values, with typed edit
and Enter events; refused oversized edits restore the accepted visible draft.
Buttons, finite choices, switches, status badges, section spacing and menus
reuse kit controls and semantic tokens without another painter.

Surface events remain requests: the view rechecks node/generation/graph, and the
future link/node dispatch must still authorize caller/context and correlate its
result. There is no product helper call, Settings route or RPC in this stage.
Native declarations choose the kit's wrapping finite-choice buttons; nested
menus replace the current core menu after selection rather than adding a
second hover-submenu renderer. These are explicit native-author-port control
choices, since arbitrary React presentation cannot execute in GPUI.

The coordinator approved an additive markdown_guarded entry with typed
MarkdownGuards, delegating to the existing Markdown renderer. It supersedes
the earlier unmerged link-only helper. Existing markdown is unchanged.
Plugin-authored links pass the canonical HTTP(S)/no-userinfo URL validator and
the supplied core opener; test windows always supply a recording fake opener.
This refuses executable OS schemes and credential-bearing links rather than
letting the Markdown component open an unchecked URL. Refusals emit safe typed
events, with no raw input in their diagnostics. New actual-input GPUI tests
exercise Enter before/after offline and valid/refused Markdown link clicks;
these cases are written but have not yet run.

The resource guard also supplies ImagePolicy::RefuseAll through TextView's
authoritative image_source hook for rendering and intrinsic sizing. This stops
implicit remote fetch, local/relative image paths and embedded data SVG decode;
link interception alone would leave those default paths reachable. Every URI
receives the same inert loader and fixed refusal, without URI capture or loader
fallback. A typed Resolve callback exists only for a future approved host-owned
contract; plugin declaration data cannot select it. New kit tests check fixed
source identity and actual Markdown hook routing for all four URI kinds against
a recording fake HTTP client, requiring zero requests. These cases are not yet
executed and no actual image decoder or network permission proof is claimed.

A separate button fixture first activates the real kit button, then changes the
sole owner to offline and invokes that captured dispatch before a toolkit
hover/pressed repaint can disable it. This distinguishes actual owner recheck
from visual disabled-state protection; neither route may publish a request.

Popover rendering is dependency-blocked on W19b's shared content popover and
its V1/lead handoff. Until that exact producer is available, popover controls
are unavailable. The coordinator expressly refused a Dialog substitution;
no private popup or modal imitation is added. The future adapter must retain
and recheck the producer's opaque Lease as well as this contribution's exact
node/generation before admission, dismissal or focus acknowledgement.

## Follow-on plugin usage adapter

The coordinator assigned F6 the usage adapter after the current body work.
It starts with a typed Ready/discover/fetch contract and versioned, bounded
safe disclosure claims bound to exact plugin node/generation provenance, then
V3 review and a real adapter into W5's single source actor. W5 owns report
values/consent/refresh; F6 owns trusted plugin registration/helper supervision.
Builtin store/recipient consent text cannot describe plugin claims. Until the
reviewed executable handoff exists, plugin usage remains Unsupported; no second
usage actor or speculative helper message is introduced here.

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

## Live panel placement

Installation retains one immutable ClientDocument. Feature owners borrow the
same contribution graph; there is no second pane/sidebar/composer manifest.
Registry::resolve_contribution rechecks exact node/plugin/generation/kind/id.
Replacing a graph under an unchanged generation is refused atomically so old
captured actions cannot gain new behavior.

Registry::resolve_panel(address, context) returns a borrowed
ResolvedPluginPanel with private fields and address/context/locations/supports
accessors. The borrow prevents retaining that policy across registry mutation.
W12 resolves before every new open/drop/retarget and restore availability
check; it persists only the target and original slot/modified state. Missing,
offline, stale or newly unsupported placement stays unavailable in its saved
slot. Workspace/agent context must match the actual declaration; shared
workspace ownership is checked by its owner. FilePanel selection belongs to
W13's existing file resolver; this workspace-panel capability deliberately
refuses File context. W13 keeps Target::File/buffer/dirty state and mounts the
selected FilePanel renderer in that existing file host; explicit plugin-file
opens route through that owner, never a fabricated PanelLocation.
No new node call, raw manifest or serialized capability is introduced.


## Staged plugin localization boundary

L's checked source/narrow seams through9fedb896 are merged, without claiming
whole F6 acceptance. A14 test-only plugins::translations stages the separate
installation namespace until a typed catalog-resource DTO and the sole app
Hub publication/language lifecycle consume it in production. No plugin keys
are inserted into core Translator and no locale RPC is introduced.

The model reuses maintained FluentResource parsing, L's scan/expanded checked
Contract/compare pipeline and the shared Arc bundle factory, Number/Date bridges
and NUMBER/DATETIME/PARTICLE/pseudo policy. English is mandatory. Canonical
language identifiers are parsed by the existing unic-langid dependency;
canonical duplicates, duplicate entries, attributes, junk, dangling refs,
wrong variable types and unbound/mistyped term arguments refuse before use.
Translations may omit English variables/keys but may not invent public keys
or types; each message separately falls back to the English plural locale.

Actual bounds live once in translations/mod.rs: sixteen locales,32KiB per
source,256KiB total,256 total entries,128-byte identifiers,16 arguments and a
128-byte control-free zone label. A conservative128 raw opening-delimiter
budget precedes the maintained parser: it bounds possible recursion without
reimplementing Fluent grammar, but literal/comment-heavy resources may hit it
earlier. This is an explicit author-port resource difference. A32KiB bounded
fmt writer caps intermediate output before final Caption/ThemeName/ContentText
validation; failed translated output validation also tries English. Both
concrete safe failures are retained if fallback fails. Arbitrary source,
contract and formatter diagnostics are sealed, including Debug/source chains.

Prepared formatters own resource Arcs across language reprepare; captions,
content and names are produced before publication, not intended for drawing.
This stage does not yet prove app language subscriptions, cancellation,
generation-fenced publication, installed cache, disposal or actual rendering.
Those remain the coupled production wiring's obligation. Tests use fictional
English/Korean resources and no copied reference translation assets.

App A8 merge now includes line2e95b7d31 and its exact reviewed scalar e46
ancestor. Add/add conflicts retained the later owned graph/JS-edge decoder
corrections and source-boundary documentation, rather than restoring the older
scalar-only grammar. No whole-node head entered the app tree. Existing narrow
evidence remains pinned to its original immutable source; merged-head checks
remain required.

## Bounded private-development catalog adoption

The coordinator authorized the same bounded development exception as W3/W14:
this private F6 app tree may consume the exact catalog value files from
3a8fa24fe, without merging any node head. The catalog, counting sink and document
sink refactor match that commit byte for byte. The decoder tests additionally
retain the reviewed ce18372bb precision correction, which changes only the
required_client JSON member rather than arbitrary digits in a random UUID.
No authenticated call, row, handler, settings payload or dependency is added.

Before building the real single app Hub, this exact tree must show a root
vessel-node build and the proto tests. No second catalog schema or parallel
Vec<Installation> transport path remains when that adapter is implemented.
Until the integrator carries3a8fa24f and an announced app base includes it,
none of the dependent F6 implementation is handed to either integrator or lead.
Current root build/proto execution is pending; this exception is not shared-line
or final package adoption.


## Retained editor event ownership

The renderer now uses the kit's existing TextField::with_source and real FieldId
instead of adding a plugin counter or inferring identity from equal text. The
parent matches every edit/submit to the current editor entity in that control's
slot before changing a draft. Reusing a control id cannot admit an old editor's
queued submission into its replacement, independently of the already checked
contribution/node/document origin. The regression keeps the old actual entity
and subscription alive, emits its queued submission through EventEmitter after
replacement, then uses real Enter on the current field. It distinguishes typed
event delivery from retained pointer-frame dispatch; execution is still owed.

The shared content-popover seam is now on the announced 2b806402 base. Its UI
lease does not replace the graph/context checks, and the F6 adapter remains
unimplemented. Before mounting editable popover content, each opening must own
and retire its field entities/subscriptions and captured input origin; an old
opening cannot submit through newly mounted fields. The existing kit anchor
probe supplies measured trigger bounds; placement and rendering stay with the
shared popover. No Dialog substitute or area-owned popup geometry is introduced.
The current fixed field-subscription collection is bounded by the declaration;
a future opening adapter must not append subscriptions across unbounded reopen
cycles. A lost or changed owner retires its body before deferred close/focus ACK.

The base merge changes no root crates/Cargo files. Pending root Node/proto
carrier checks on2fa4b832 therefore still cover the identical root source;
new native consumers require their own current combined verification. The
private-development-only handoff restriction remains in force.
