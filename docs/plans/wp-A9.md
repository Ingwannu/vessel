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

The borrowed catalog presentation now distinguishes never requested, the
node's still-unfetched cache, initial loading/failure, fetched empty,
filtered empty and nonempty ranked rows. A successfully fetched copy stays
visible during refresh/failure, with a retained failure said only once;
an unfetched default reply cannot masquerade as successful empty data.
This module owns no second cache or request channel. The existing page must
still supply only its own node/ticket-accepted copy when the reader is wired.
These network states are a necessary difference from Paseo's static ACP
catalog; loading/error furniture follows ProviderModalBody 488–570.
The new Korean registry words use “프로바이더”, matching the existing
Settings section label rather than introducing another noun for that page.
The empty sentence describes what this machine listed, not a claim that the
public index contains nothing: the node may have omitted unreadable or
unpinned entries. Its aggregate skipped count remains in the accepted copy;
the node does not give individual omitted entries or per-entry reasons.

## Resume point and verification

The configured-route Models candidate and protocol-admission compatibility
repair are committed. Exact-source narrow compiler/runtime results and
exploratory Models checks are recorded below; the new delegated shared
limit still needs its changed-head checks. There is no A9 whole-gate,
final capture matrix or completed-package claim. The canonical comparator
has matching-source narrow runtime/compiler evidence and was handed off
for reuse without claiming final package review.
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
`view::prefs::Choices` validator. The original preflight introduced no dependency, preference store or
writer change. The delegated shared bound update below now aligns validation
and persistence without changing other preference behavior.

The private Prepared vector exposes only its checked slice. Unknown,
wrong-kind or ambiguous choices are refused before the writer; ids/values
at the shared 256 UTF-8-byte limit and lists at eight entries retain the
current contract. The prior 64-byte guard was too short for real model ids.
The area closes the picker and draws the shared error Alert on refusal,
keeps previous preferences, and sends no SavePrefs command on that branch.
A successful choice/reset clears the error; changing machine/agent or
leaving the Providers section also clears its context. New whole EN/KO
sentences use generated `agent-defaults.ftl`.

Tests cover a long but actually offered model, unchanged prior choices,
default-value omission, byte/count edges (including Korean UTF-8), stale or
wrong-kind choices, repeated ids and every whole translated sentence.
The initial preflight left the writer's limit unchanged and made refusal
visible. Its exploratory callback/file evidence and the later delegated
shared limit update are recorded below; neither is full A9 UI approval.

The first defaults Clippy run returned documentation-markup and explicit
test-constructor lints. The correction quotes the existing type path and
names SignInHelp directly, without weakening the lint or changing product
decisions. That red run is kept separate from runtime results on the
earlier Models/catalog source.

The next Clippy pass reached App and required the default-save helper's
machine/agent arguments to be borrowed. Its caller already owns those local
values; the correction changes neither context checks nor persistence order.

## Package base 3 integration

Merged the assigned `6c782cb6` (Quality B and the owner-supplied web race
fix). The previous exact-source narrow check remains attributed to its
older commit, not this merged tree. Root changes belong to that assigned
base; this app package does not edit node behavior or web tests.

Models no longer repeats the version table: the typed client owns protocol
admission before a frame/request id. A small private list adapter turns only
its definite NotInProtocol refusal into OlderNode; unassigned contracts,
permission/transport failures and actual node refusals stay typed failures.
No missing version is replaced by a dummy number. The existing no-frame
older-peer tests and request order are retained. The removal worker already
has its separate explicit protocol guard and typed-client refusal handling;
no removal product code or deadline changes in this compatibility update.

Package base 4 (`3cd0d3fc`) is also merged as assigned: published language,
Settings frame, Devices, sidebar, Git and terminal changes remain intact.
Own Models registrations auto-merge into their existing dispatchers. The
README retains both sets of module notes; the size list keeps slider 711
and the base's lower demo 517. The root source is the assigned base's,
not a node change from this app package. Narrow checks below still need
to cover this new merged tree before any current-source claim.

## Delegated shared choice-byte limit

[Decision Log]
- Purpose: a real offered 67-byte model id must save without truncation.
- Existing constraint: View and Link separately limited saved option ids and
  select values to 64 bytes. The visible refusal guard prevented silent loss,
  but did not make a valid long model persistable.
- Alternatives: silently truncate opaque ids, maintain per-area storage, or
  let the validator and writer keep separate limits.
- Choice: the coordinator delegated one `prefs::MAX_CHOICE_BYTES` constant,
  set to 256 UTF-8 bytes and used by both the validator and existing writer.
  Paseo stores provider ids as given; real ids exceed 64 bytes.
- Why and effects: the same representability contract covers Settings and
  ordinary saved choices. The count of eight, file format, writer filtering,
  atomic save, permissions and every other preference remain unchanged.

The real-file regression follows the actual fake agent's 67-byte offered
model through production preflight, keep_choices, atomic save and reload.
A separately offered 257-byte value produces the existing translated error
Alert meaning and leaves the prior preferences/file unchanged. Byte-edge
fixtures cover 256/257 for both identifiers and values, including Korean
UTF-8; count and stale-option rules remain intact. Narrow checks for this
changed head are pending; no UI capture on this new bound is claimed.

Earlier exact-source evidence remains attributed to `76557b9b`: changed-crate
Clippy passed, then test run `agents-narrow-20261115T084535Z-76557b9b` passed
1,215 tests with zero failures and two inherited ignores, including two doc
cases. View, Kit, Link and App actually compiled. The local debug app then
built after preparing the three already-required code faces. Exploratory
EN/dark images at 1280 and 390 were opened personally: true empty and actual
lists differ, query filtering works, and an actual narrow copy returns the
complete 133-byte model identifier. A short default saved as `large`; the
old 67-byte refusal visibly retained that prior value/file (0600). That last
image documents the old limit, not the new 256-byte behavior or final A9 UI
approval. Final whole-package gate, complete matrix and V1 review are owed.

The first changed-source local Clippy returned 101 on the new test helper's
`unwrap()` outside a `#[test]` body. The helper now returns its IO error;
assertion sites inside the two tests keep the original checks. That red run
is retained. The narrow check on this corrected commit will run remotely,
as required by the operator's 10:3x build-only-on-VM rule; no whole A9 gate
or new UI proof is claimed here.
