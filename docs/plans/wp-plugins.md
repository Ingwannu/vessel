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

The baseline 1d1f11df has no plugin RPC. Protocol-23 app base is being
prepared separately; merge its announced reviewed head under A8. Native
pure work can proceed. Runtime/installation/catalog calls and user-visible
controls wait for shared-contract authorization; no feature is advertised
as complete until its real path works. The package is not a whole-package
gate or runtime/capture proof merely because its declaration tests pass.

Coordinator subsequently authorized W16 to implement both sides in a
second node worktree based on d954fa3a. The integrator confirmed one pending
PLUGINS Option None and one settings namespace; no SQLite schema is needed
for owner-private settings/source files. Quality B is still awaited before
method registration. Workspace identity belongs to W11 ADR 0082, pane/tab
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
