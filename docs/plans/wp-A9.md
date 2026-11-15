# Providers and models in the native app

## Structure and decision

This package extends the existing Settings Providers page and Models page,
not a second navigation group. Pure rules remain in `view::agent_status`
and `view::models`; bounded typed node calls use the existing agent hub and
the Models hub in `link`; Settings areas only render and dispatch. Text
uses generated Fluent scopes with English and Korean files. No new crate
dependency or external API is introduced by the app package.

[Decision Log]
- Purpose: carry the provider and model settings behavior into the native
  app while showing only facts the selected machine actually reports.
- Existing assets: Settings frame, agent readiness/default rules, typed
  matcher, copy controls, alerts, loader and bounded node request hubs.
- Constraints: the current node lists configured model routes and explicit
  profile references, not a general provider/model compatibility matrix.
  Expanded provider discovery/diagnostic contracts are separate node work.
- Alternatives: invent provider capability tables, copy another owner's
  controls, or infer a reconnect from an unchanged Fleet snapshot.
- Choice: reuse those modules and the canonical actual-connection notice;
  implement only numbered callable contracts and retain typed refusals.
- Why and effects: source ownership and one rendering path stay intact.
  Missing contracts block their own surfaces, not independent read-only
  models or catalog rules. This package is not complete yet.

## References

Paseo 0.15.4 `components/provider-diagnostic-sheet.tsx` (models 40–75,
body 488–570, search/header 575–749, row 787–808) and
`provider-diagnostic-models.ts` define the model list behavior. The current
configured-route page's differences, data flow, tests and fixture shapes
are recorded in `backend-m12-models.md`.

`components/provider-catalog-search.ts` (13–39) and its complete four-case
test file specify catalog ordering: alphabetical without a query;
otherwise match visible title/id, relevance first, title tie. The existing
`view::matcher` owns scoring. `components/provider-catalog-list.tsx` owns
the catalog tile/control geometry; its shared controls remain separately
owned and are not recreated in the area.

## Shared alphabetical comparison

`view::alphabetical::compare(&str, &str)` extracts the composer's existing
Unicode `str::to_lowercase` comparison followed by original-string order.
`composer::slash::ranked` now calls it only after relevance ties; empty
queries still preserve the agent's command order. The standard library
already implements the required rule; there is no added collation
dependency, per-area comparator or normalization pass.

Tests retain the six original provider-catalog titles and expected order,
case ties, Korean, accented/canonically distinct forms, supplementary
scalars, expanding lowercase and contextual Greek sigma. Original UTF-8
string comparison preserves Unicode scalar order, not UTF-16 unit order.

Open question: this deterministic order deliberately differs from Paseo's
locale-tailored `localeCompare` with sensitivity `base` for accented names
or tailored non-Latin scripts; adopting a data-backed collator is deferred.

## Registry rules prepared before the screen

`view::agent_status::catalogue` consumes the node-resolved `CatalogueEntry`,
not the public index format. Its fixture mirrors the resolved reply from
`crates/vessel-node/tests/e2e/catalogue.rs` (pinned npx package/args/env and
one skipped floating version). There is no app-side registry parser.
The full four Paseo search cases use their original six titles/id inputs;
extra cases preserve score priority, original-case ties and title/id-only
matching without treating a description as a hidden search keyword.

Eligibility matches node `added.rs::from_registry`: only Command can be
recorded. `runner_found=false` is help before starting a chat, not a reason
to disable recording. Known agents, downloads and unavailable platforms
remain separate typed outcomes. An explicit registry source with the same
id marks an already recorded entry regardless of display name/version;
matching a command by name or argv does not prove that origin. EN/KO help
is generated from `agent-catalog.ftl` with whole-sentence tests.

The coordinator's registry requirement retains an already-added mark;
Paseo's catalog component instead filters registered provider ids out.
That is a deliberate presentation difference, not proof that the two lists
have identical membership. Node-run eligibility remains authoritative.
Recording an entry does not execute or install it. No mutation producer or
screen is implemented by these pure rules yet.

## Resume point and verification

The configured-route Models candidate and protocol-admission compatibility
repair are committed. Narrow compiler/runtime checks are still in progress;
there is no A9 whole-gate, Models capture or completed-package claim.
The comparator/source extraction is prepared for its narrow checks.
The isolated shared connection-ready producer is consumed without a menu
dependency. Models adds one typed book-key/physical-node notice to that
bounded fan-out. It clears current cached facts and pending reads on every
real reconnect, even the same node id; an unrelated machine is untouched.
The never-reused ticket sequence survives. Tests cover same-id reconnect,
node replacement, late replies, unrelated notices and navigation. This is
not proof of retained clients or Read permission: ordinary request results
still decide those. Notices occur before the sessions query and final
Fleet publication, preserving the producer's original timing.
Provider diagnostics, richer per-provider model/mode metadata and enable
state await the separately reviewed numbered node contract. Catalog Add
and ProviderRow action controls await their existing shared-control owners.
Named launch-profile CRUD/application remains a later package and is not
implemented against a draft contract.

The first narrow catalog candidate failed compilation because its HttpUrl
import used a private module instead of the protocol crate's existing
public re-export. The correction changes that import only; no visibility
or dependency is widened. The failed run remains red evidence, and no
runtime test or UI result is inferred from it.

## Package base 2 integration

Merged the assigned `6036eeb8` after cancelling the still-queued local
narrow check; its exit 143 is not a compiler result. Root source is
unchanged by this base update. The shared `Machine.node` now carries the
welcome identity for a new page selection; actual reconnect invalidation
continues to use the single connection-ready fan-out. The shared field's
event extraction and typed Editability API are retained, not reimplemented.
The size-list merge preserves this branch's measured slider 711 and the
new base's lower field 635/demo 583. No allowance is raised.

The Models area passes the shared `Machine.node` into a new selection's
pure state. A later render of the old Fleet must not undo an earlier real
connection notice: the shared producer intentionally emits before its
new Fleet. A regression checks old-Fleet render → new-Fleet render while
the replacement request remains pending. Same-key connection changes still
use only the canonical fan-out, not Fleet-difference inference.

Each Models snapshot additionally carries the completing client's actual
handshake id. A page can open after an early notice was ignored while the
page was hidden, before the new Fleet is published. Its accepted read must
confirm the actual id instead of retaining that earlier Fleet's id. Only
an accepted machine/ticket changes this metadata; a stale successful reply
cannot change it. This adds no connection hook, counter, request or wire
field. The in-memory link test checks the read's id against its real client.

## Default-choice preflight

The legacy default-choice writer silently filters ids/values past its saved
bound. The page now prepares the complete proposed choices in
`view::agent_status::defaults` before touching preferences. It reuses the
existing option eligibility function (the only shared-code seam is its
private-to-crate visibility), Picks/choices_for behavior, and
`view::prefs::Choices` validator. No new dependency, preference store, bound
or change to the global writer is introduced.

The private Prepared vector exposes only its checked slice. Unknown,
wrong-kind or ambiguous choices are refused before the writer; ids/values
at 64 UTF-8 bytes and lists at eight entries retain the existing contract.
The area closes the picker and draws the shared error Alert on refusal,
keeps previous preferences, and sends no SavePrefs command on that branch.
A successful choice/reset clears the error; changing machine/agent or
leaving the Providers section also clears its context. New whole EN/KO
sentences use generated `agent-defaults.ftl`.

Tests cover a long but actually offered model, unchanged prior choices,
default-value omission, byte/count edges (including Korean UTF-8), stale or
wrong-kind choices, repeated ids and every whole translated sentence. The
actual callback, disk persistence and captures still owe execution; no
writer fix, saved-success acknowledgement or storage-contract change is
claimed. This explicit refusal is a safety difference from the old silent
filtering path, not a claim of larger model-id persistence support.

The first defaults Clippy run returned documentation-markup and explicit
test-constructor lints. The correction quotes the existing type path and
names SignInHelp directly, without weakening the lint or changing product
decisions. That red run is kept separate from runtime results on the
earlier Models/catalog source.

The next Clippy pass reached App and required the default-save helper's
machine/agent arguments to be borrowed. Its caller already owns those local
values; the correction changes neither context checks nor persistence order.
