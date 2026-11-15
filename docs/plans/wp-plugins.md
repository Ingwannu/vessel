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

The initial baseline 1d1f11df had no plugin RPC. The app tree now merged
package base 8d5513e4 and the node tree merged Quality B 29eebb8b under A8,
both without conflicts. No feature is advertised
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

The app tree merged package base 8d5513e4 without conflicts; node work
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
