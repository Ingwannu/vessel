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


## Registry read connected to the existing page lifecycle

The reader follows `provider-catalog-list.tsx:104–177` and the existing
node catalogue contract; it does not parse a second public index in the app.
It reuses the Agents page ticket, bounded Hub and the actual connection
notice array. Node identity and ticket checks reject late, duplicate and
book-name-reuse replies. Same-node reconnect invalidates cached reads;
install output retains its physical owner even after completion clears
inventory. A registry failure is translated separately from a removal.
The node's HTTP budget is30seconds, so the client read budget is45seconds;
ending that wait does not claim the server fetch was cancelled.

[Decision Log]
- Purpose: expose platform-resolved registry facts without another state store.
- Existing constraint: one pending operation per machine, four Agents workers;
  the record-only Add contract must never automatically probe a new provider.
- Alternatives: a new registry actor/cache or an automatic discovery after Add.
- Choice: add Catalogue to the existing typed operation and accept its reply
  only for the retained inventory's actual node. Add will consume its ACK
  directly; no install, discovery or options probe is implied.
- Why: book labels can be reused and cached empty data cannot mean fetch success.
- Impact: four bounded memory-transport regressions and pure state regressions
  cover fetch intent, no-frame refusals, typed errors, deadline and reconnect.
  These are new sources awaiting narrow checks; registry/Add UI is still absent.

The existing ProviderRow spinner now consumes the reviewed shared
`loading::activity_indicator` numeric10 profile (750ms), preserving its
semantic colour. This replaces the legacy800ms slider profile; no new
spinner/vector/control is added. Actual first-use phase/captures remain owed.
The shared button and menu registrations arrived through lead9e18cf0d;
that merge kept all92 glyph variants and their matching SVG paths.


## Record-only Add intent and call

The original form retains book route, physical node and (for registry entries)
the full displayed pinned offer. Validation rechecks these facts, parses the
existing node name/program types, refuses case-insensitive name reuse and
always sends replace=false. Registry ids are not truncated to the shorter
agent-name contract: the form will ask for its own editable valid name.
Command arguments are literal lines, with no shell parsing; no environment,
secret, inherited variable or MCP grant is added implicitly. The node remains
authoritative for the shared profile namespace and Admin/local-person policy.

A valid form reserves the existing page ticket; the link consumes its private
request and checks the actual client node immediately before the existing
numbered AddAgent call. A30second local deadline or lost/malformed ACK is
uncertain, not an automatic retry. The original typed wire error is retained.
An accepted ACK must match book, ticket, operation, physical node, name and
source/pinned version. It appends only that definition; readiness remains
unknown until a person checks. No Reload, discovery, status command, options
probe, installer or package runner follows Add success. The existing shared
definition-freshness state blocks another mutation after an uncertain write.

Pure tests cover literal arguments, invalid/reserved/duplicate names, explicit
registry name and missing runner, changed node/offer, wrong ACK correlation,
duplicate ACK and uncertainty. New finite memory-transport tests exercise the
actual Hub, typed no-frame refusals, original node refusals, mismatched ACK and
30second virtual deadline/no resend. Catalogue and addition tests share one
bounded peer; catalogue success mode explicitly panics on a mutation. Product
removal code and its previously accepted tests are unchanged.

These sources are committed before UI wiring and are not yet compiled. Both
previous remote narrow submissions were cancelled by signal at10:35:42 while
still queued; neither ran Cargo. The cancelled runs remain143, not failures or
passes of these newer sources. Tool owner was asked for retry disposition.
No whole package gate, new capture family or completed Add UI is claimed.


The first A9 source narrow on e2afa5df returned101/step1 at View: two input
parser failures lost their typed sources and Failure::variant used a wildcard
arm. Refused now retains AgentNameError/InvalidAgentText as sources, and the
alert match is exhaustive. This is a source repair, not lint suppression;
whole localized form instructions still select by typed field/category. The
new native memory fixture is directly cfg(test) with the same native-target
guard, keeping test-only helpers within the workspace's test lint policy.
The old143 runs were operator-approved VM-resize interruption (T confirmation),
not a source result; their latest replacements were not duplicated.


## Existing check/install identity boundary

While joining Add to the existing actor, inspection found that the older Check
and Install variants retained only a book key. `dispatch.rs` resolves that key
to its current client, so a repaint/reconnect race could redirect a status
command or offered installer to a different physical node. The displayed
inventory's NodeId now travels with both variants and is compared with the
actual client immediately before the first action call. A mismatch sends no
check/installer, keeps the original expected/actual IDs as typed error fields,
and uses the existing translated NodeChanged instruction. The new bounded
Hub regression proves both refusals and an ordered no-frame barrier. Normal
install progress, polling, retained output and offered command are unchanged.
This adds no RPC, credential, permission grant or node behavior.


The75b74ac1 narrow reached actual View/Link compilation and returned101 on
test scaffolding: the concrete memory transport needs its supertrait methods
through the existing generic FrameTransport bound, and a fallible fixture
helper must return a diagnostic error instead of assert inside Result. The
repair uses the already accepted generic transport pattern and keeps the
catalogue-acceptance condition mandatory. No new dependency, allow/expect or
product fallback was added. View/Link tests did not run in that red attempt.
Base6 was then merged (18eedad4): Models and RetireConnection unsent arms both
remain, and the Models hub moved into the lead's single shared test-state
constructor rather than retaining a duplicate builder. G2's exact reviewed
ProviderRow/modal producer four-commit range was adopted; no other F1 source
or future hierarchy work was consumed. All actual consumer UI remains pending.


### Add admission and authoritative registry acknowledgements

[Decision Log]
- Purpose: never present a local inability to send as a machine rejection.
- Constraint: the client distinguishes an unassigned method, exhausted local
  request ids and an actually older negotiated peer before any frame is sent.
- Choice: retain those sources and map them to distinct build-unavailable,
  restart-required and older-node instructions. No automatic replay.
- Alternative: one generic unsupported/rejected label, which wrongly tells
  people to update the remote machine or edit a valid command.
- Impact: no protocol, file writer, permission or installation behavior changes.
  A registry command also requires an actually fetched copy, not entries in an
  unfetched reply. ACK tests reject another registry id or version while keeping
  the original pending ticket until the exact acknowledgement arrives.

These source refinements are unverified until the next changed-head narrow and
final A9 gate. The earlier immutable narrow on 039c9e86 excludes this refinement.
The two priority runs on cbdd/de809 refused the stale base6 lock before compiling;
039c9e86 merges the lead's canonical lock repair dd13ce37. Those refusals are not
compiler or test passes.

### Modern provider identity boundary (coordinator ruling)

The modern provider snapshot and legacy agent definitions are separate sections.
Models, diagnostic and enable use only the modern entry; installation, sign-in,
defaults and remove use only the node's returned HarnessRef/profile facts. Neither
provider id nor driver kind establishes an account binding. The provider actor
must supply an explicit typed binding and known sign-in verdict before the page
can combine them into one row per provider as Paseo does. This is a deliberate,
temporary parity difference, not a fabricated compatibility/account table.
The provider feature remains unassigned and uncallable until its integrator
announces assignment and the canonical app base carries it. The earlier24
reservation was superseded;24 belongs to host diagnostics.


### Registry, Add and legacy Remove consumer wiring

Source follows `components/provider-catalog-list.tsx:40–105,166–232` and
`providers-section.tsx:100–173,399–437`. Registry tiles use the node's resolved
commands, the shared matcher/alphabetic comparator, measured300-point columns
(max4), source PackagePlus20/frame36 and checked Small28 outline/loading buttons.
The shared ExternalLink is only provisionally consumed after V1's bounded source
and exact Kit narrow pass on c40714c1; its first-use pixels are not approved yet.

Necessary differences: the legacy protocol records a provider without running it,
so Add opens an editable-name confirmation and a literal program/arguments form,
then sends exactly one existing agent.add. Registry ids are never truncated into
names. Arguments are one per line without shell parsing. The confirmation shows
the actual launch argv and public registry environment separately. No installation,
status/options read or reload follows its ACK. Missing package runners remain
recordable with the machine's explicit help; archives have an actual Download
link instead of an invented install URL. Already-added entries remain visible and
marked, rather than being filtered as in Paseo, following the owner's requirement.

Only returned legacy Added definitions get a Remove menu, using G2's reviewed
ProviderRow trigger and Regular16 trash glyph,220-point menu, then the existing
adaptive confirm dialog. No passive gap is reserved for builtin rows. Every
callback retains its exact decision; the non-cloneable view confirmation is
consumed once. Cancel/close cannot affect a newer dialog. No package uninstaller
is invoked; node Admin/profile-in-use refusals remain authoritative. The node's
ACK settles only its correlated page ticket; uncertainty still requires discovery.

The registry and mutation forms use new English/Korean Fluent messages, with whole
refusal tables. Shared search ownership was assigned to this track after the
preceding field writers. The installed Providers catalogue and Models consumers now use the checked shared
SearchField after V1's bounded source/Kit-narrow disposition on13c46693. Both
keep the same localized placeholder, existing change callback and programmatic
machine-selection reset; one horizontal rail lets the source max420 field shrink
without vertical flex growth. Source Search/X/clear/focus furniture stays in Kit,
not in either area. This consumer change still needs its own compiler and final
first-use evidence; the shared control pass is not a page pass.

All this consumer source is unverified until the changed-head remote narrow and
final A9 gate/captures. Earlier039c runtime evidence excludes these new controls,
messages and tests. The modern provider feature remains unassigned and uncallable pending canonical base;
no profile/account binding is inferred and A10 remains unstarted.


The039c View/Link runtime narrow returned RED:1113 passed/1 failed. Link110/0
included the real67-byte save/load and257-byte visible refusal/prior-file tests.
View1003/1 failed only the pre-existing prefs::an_agents_choices_are_bounded case
which still expected65-byte option ids to be rejected. The assigned common bound
is256 for both option ids and selected opaque values. Retain its8/9 choice-count
assertions and replace the obsolete65 rejection with literal65/67/256 acceptance
and257 rejection. No product limit or assertion is weakened to mask a failure.
Corrected current-source runtime remains owed; neither this red run nor the
new consumer's queued Clippy is a whole A9 or current-feature pass.


### First consumer compilation correction batch

The exact cbb Clippy returned RED before App: the root command dispatcher
crossed its100-line lint, one memory-test future crossed16KiB, and the fixture's
root Call catch-all violated the wildcard enum lint. Move only the existing
Agents forwarding block into its cohesive module (same real root caller and
lookup/effects), box that test's safety future, and use a method-specific fixture
whitelist. Read and Add refusal/wait modes are now distinct: an unexpected write
cannot satisfy a registry error test. Every unexpected method still fails.
No lint exception, product timeout, request bound or assertion was weakened.

The5845 runtime then stopped at compilation: generated dynamic Fluent messages
return Result, not String. Retain preparation failure as typed FormatError;
prepare a confirmation title at its input event and cache only the resolved
string. Do not format it in Render or discard its error. A failed preparation
shows one whole translated local error and never creates a confirmation or write.
Language changes reword that retained decision and update Add placeholders without
replacing editors, input, focus or intent. The only non-area app registration is
Shell::refresh_agent_words in language.rs's existing placeholder refresh list.

View also owns Add control admission/pinned-entry progress, removal action busy
state and notice tones; the app passes their typed result to shared controls.
Whole translation/tone cases cover local refusal and formatting failure. Generic
operation failure no longer promises a machine message “below”: original prose
is retained only in the diagnostic tooltip, so that earlier sentence was false.
These corrections require a new exact changed-head narrow; old reds remain red.


The requested pre-priority non-Cargo sweep checks all four native crates with
the pinned standalone rustfmt, plus size, boundaries, tokens and whitespace.
The import/API review compares the new area files with their real shared kit
exports; this is source inspection, not Rust name resolution or a compiler pass.
A doc-markdown scan found seven comments naming code without backticks; this
follow-up changes those comments only. The already reviewed ExternalLink control
is unchanged. Runtime tests remain queued at the prior behavioral source; no
priority approval for that old hash is silently transferred to this follow-up.


### Current App lint correction

The package-base8 narrow reached App and returned RED with14 lint diagnostics.
The earlier manual import scan was insufficient: the compiler found three
wildcard imports, an unused trait, unnecessary badge qualification and an unused
form query. Remove those, retain explicit imports, add required unit-statement
semicolons and express missing/mismatched inventory with is_none_or. These
changes keep the same action guards; they do not relax them. Split only the
stateless catalog name/resource furniture from the offered action path, retaining
its original child order, values and one shared control renderer. This closes the
119-line function without a lint exception or a parallel widget. No new test is
added for this behavior-neutral cleanup. The old run remains red; the corrected
App-only narrow is a new immutable-source check, with no extra priority assumed.
The existing624 View/Link runtime ticket remains queued unchanged.


### Current compiler and protocol status

Exact0625a619 App all-target locked Clippy is now0, native-fonts0/no skip:
agents-20261115T133138Z-0625a619. The log actually checks Kit, Link and App;
this is the requested App narrow result, not an isolated whole Link test run
or an A9 gate/UI pass. Steps SHA256:
52f0349323f3f8e1464612436c822a958ba06b6521854a3bb4a533d17df2977c.
The earlier14-diagnostic App run remains red. The624 View/Link runtime ticket
completed without being moved or canceled; new SearchField producer checks are separate.

Provider numbering correction from the node owner:24 is now assigned to host
diagnostics. The provider feature remains None/pending until its actual wiring
merge; do not use24 or guess25. The staged typed schema does not authorize a
call. This app still contains no modern provider RPC registration or inferred
legacy binding. Node actor binding/sign-in and the queued original-lease product
promotion remain separate prerequisites. The reserved E2 Status8 control has
no implementation/hash yet; the prior visual dot override is baseline only.

SearchField exact2e3f05bf received a complete V1 bounded SOURCE PRECHECK without
a blocker, with explicit consumer/adoption hold pending exact Kit Clippy and
two focused runtime results. Its authorized temporary agents-control label runs
those sequentially without priority and is retired after handoff. Providers and
Models consumer changes remain prepared outside the tree until that condition.
The next shared field writer after the checked handoff is G2 for typed code text.


### Completed corrected View/Link runtime

Exact62478a9e locked View/Link tests completed0 at run
agents-narrow-20261115T131245Z-62478a9e, fonts0/cargo0/no skip. Raw totals:
Link111, View1019, localization10, matcher3, page117 and View doctests2;
1260 ordinary tests +2 docs,0 failed/ignored. Both changed View and Link
actually compiled. Steps SHA256:
178577e3de4a2389e679fbdf7e38a65f67a63196ac9f9aef861c9a238446eac4.
The67-byte real-file roundtrip/0600,257-byte visible refusal/prior-file retention,
corrected choice-bound test and full removal/format-failure sentences explicitly
passed. This closes the known old runtime red for those sources; it does not
replace final A9 integration/UI/authority/capture evidence or SearchField tests.

The shared bound producer8fd depends on this package's defaults preflight and
prepared Fluent domain, which are not on package-base8. Its test correction5845
retains literal count and byte boundaries. Do not hand8fd to another consumer as
a standalone patch without those prerequisites or an approved isolated handoff.
The preference owner received these exact results and the dependency warning.


### Local screen callback regressions

Pure view tests cannot exercise the screen's retained editor and Rc callback
identity. New test-only App cases use the existing isolated Shell constructor
and bounded Link::detached command recorder, with no actor, node, updater or
filesystem. A stale Add form cannot close/submit its successor; a sending form's
actually focused editor refuses keyboard edits; closing leaves the sent ticket.
An old machine's successful ACK keeps the newer machine's form and emits no
options probe, discovery, install or reload. An old removal decision cannot
dismiss/confirm a newer decision, and repeated confirmation records one command.
This adds no production action path or visibility widening. The fixture reports
no transport/scopes/lease: these tests do not claim real authorization, current
lease fencing, native mouse geometry or screenshots. Expected behavior would fail
without the existing actual callback/read-only guards. Exact App narrow results
for these new tests are still owed; older624/062 results exclude them.


The exact7fbbb App all-target Clippy returned101 for three missing
unit-statement semicolons in the new test callbacks/initialization. No product
diagnostic was reported; the sequential runtime stage did not start. Correct
those three statements without changing assertions or handler behavior, then
run the corrected changed-head App narrow. The red run is retained and is not
a runtime result. The announced green c3429952 base was meanwhile merged with
only a README append conflict; its shared boxed SavePrefs command and field
predecessor are preserved. No complete A9 or current-screen pass follows from
that base's gate.


### Checked shared search consumption

V1's exact13c46693 bounded Source/requested Kit narrow pass permits selected
SearchField adoption. G2 received the next shared-field writer baton for Code
form text; this package makes no further Kit field edits. Temporary agents-control
is retired. Its one approved focused-runtime priority was consumed once after
the old queued ticket reached143; no priority is carried into these App changes.
The Models and catalogue areas replace only the previous ordinary field/API and
horizontal search rail. The query still comes from the same editor engine, clear
uses its regular Change path, and navigation resets stay silent. No controller,
node call, extra input engine or consumer-owned search glyph is added.


### Discovery completion owns its follow-up

Source review found an area-level redirect: a List completion always called the
current page's initial-catalogue helper, even when the view rejected its ticket
or accepted it for another machine. If the newly visible page had a cached
inventory but no registry read, that older response could initiate new I/O on
the new selection. This is separate from the still-unimplemented canonical
DeliveryGuard: it needs no new actor, RPC, lease or per-area epoch.

Pages::loaded now returns a must-use acceptance verdict from its existing
machine/ticket/operation gate; the mutation body is unchanged. The App starts
its initial catalogue read only after that verdict AND an exact current book
match. Existing fixtures assert accepted/rejected results (including stale and
wrong-operation cases) without dropping any prior assertion. New I/O-free real
screen-handler tests cover accepted A after switching to cached B, a duplicate
consumed discovery, and the positive current reply still issuing exactly one
correctly addressed catalogue read. These tests do not claim transport scopes,
queued-lease fencing or a real node request. Changed-source checks remain owed.
