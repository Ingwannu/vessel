# A7 acceptance map

This is the current integration index for the detailed survey and decision logs
in [wp-timeline.md](wp-timeline.md). It distinguishes the running compatibility
path from typed preparation. No final A7 gate, capture set or V4 approval exists.

## Responsibilities and data flow

The four native crates retain their existing boundaries. `view` contains pure
projection, identity, find and display rules; `link` owns node calls and bounded
transport; `kit` owns controls and the shared text/output engine; `app` renders
the existing conversation resources. GPUI never decodes provider JSON into new
semantic tool kinds.

Existing record delivery enters the conversation's exact-session `take` guard,
then the one `fold_records` body and `view::timeline::Timeline`. Rows, disclosures,
find and source-membership maps refer to that projection. `timeline` and the
compact `tool_sheet` render the same `entries` switch, tool body and text pacing.
Copy/find retain full canonical assistant text; pacing changes its shown prefix.

| Responsibility | Existing source extended | Current product/preparation boundary |
|---|---|---|
| Canonical sequence membership | `view/src/timeline/{entry,source,fold,append,user_stream}.rs` | Product legacy events; Entry.through is an exclusive upper bound, ExtraSources gives exact membership. Neither is a request delivery revision. |
| Tool display and typed sections | `view/src/timeline/{tool_source,tool_display,tool_sections,tool_status}.rs` | ToolView supplies legacy state; its detail/failure/provenance methods intentionally return None until W23's field/scope union. Canonical fixtures prove model behavior only. |
| One tool body | `app/src/areas/conversation/{tool_body,tool_detail}.rs` | Main A7 uses one kernel; portable A8 facade stays staged until its real caller and legacy delegation are enabled together. |
| Diffs | `view/src/timeline/diff/`, `kit/src/output{.rs,/}`, `tool_body.rs` | Existing virtual output/copy with bounded line and intraline changes. K's checked minimal syntax resolver is composed only in a separate unverified syntax preparation, not this main tree. |
| Plan and tasks | `plan_card.rs`, `plan.rs`, `view/src/timeline/plan*` | Shared Markdown card; W4 owns pending approval actions. Stable task id/active_form await W23. |
| Turn/tool/reasoning disclosure | `view/src/timeline/{turns,runs,disclosures}.rs`, conversation `{turns,disclosures,tool_sheet}.rs` | Consumed by the existing Chat preferences and rows; full followed log remains available to W1. |
| Footer and context | `turn_footer.rs`, conversation `{footer,meter}.rs` | Known-range duration, failure suppression and full reply copy; exact context danger boundary is >90 percent. W5 owns account limits and additive usage children. |
| New notice/compaction/plugin rows | `view/src/timeline/{items,compaction}.rs`, conversation `notices.rs` | Typed projection/renderer preparation. No activated rich AgentEvent intake. Plugin fallback names plugin/kind and never interprets opaque data. |
| Mounted text reveal | `view/src/timeline/text_reveal.rs`, conversation `pacing.rs` | Product assistant/open-reasoning prefix, first-sight hydration, grapheme safety, reduced-motion/find bypass and weak bounded frame wake. Source-scoping regression executed at4b591a90; later changes remain separately bounded. |
| Spoken prompt display | `prompt_presentation.rs`, `searchable.rs`, conversation `{bubbles,timeline}.rs` | Product source uses one borrowed display slice for user bubble, local search, copy and rewind text. View strict/tests executed at6d8c0cd8; canonical body is untouched. |
| Submission ledger | `view/src/timeline/pending_messages.rs` | Pure cap32 typed preparation only. Legacy Tail is still the sole product optimistic path. |
| Resident ownership | conversation `resident{.rs,/}` | Literal cfg(test) preparation over W12's sole registry, actual Area snapshot move and Clone context Seat. Product Copy Seat/Place/globals still require coordinated conversion. |

Paths in the table are relative to `spikes/native/` unless a full prefix appears.
Words use existing Fluent EN/KO catalogs, including approved Chat reasoning and
timeline domains. No second localization or preference store exists.

## Technology choices

Rust ownership moves the original Area and controllers in the staged snapshot;
it avoids reconstructing fields, query tickets, focus and scroll from a transcript.
It deliberately retains memory rather than claiming serialized cold reclamation.
GPUI keyed state/weak frame callbacks own only drawn text reveal counters; no
detached session timer or second session registry exists.

The already-locked `similar` library supplies bounded line changes; the existing
virtual output engine renders and copies them. The already-locked
`unicode-segmentation` version becomes a direct view dependency for grapheme-safe
UTF-16 reveal units. Existing ECMAScript whitespace matching and standard borrowed
string operations implement voice display, without a new regex parser/dependency.
Kit attribution/About registries must list both direct dependencies.

## Checked evidence and its limits

| Immutable source | Execution | Limit |
|---|---|---|
| Main e66d658b, base13 | App155/0/2 + Kit255/0/0 =410/0/2, run `wp-timeline-models-20261116T003003Z-e66d658b` | Actual current-source App/Kit execution; no current all-crate strict, View-only rerun, whole gate or UI/V4 inferred. |
| Portable body8737a401d | Executable tree equals tested289cf10a except why-doc; strict View/Kit/App0 and full tests1530/0/2, runs `wp-timeline-narrow-20261116T003002Z-289cf10a` and `wp-timeline-state-tests-20261116T003002Z-289cf10a` | Actual same-byte format/warm-cache and empty-patch regressions pass. V1 row276 bounded successor recheck pending; literal staging remains and no A8 activation is claimed. |
| Portable body5c9636a0 | Exact View/Kit/App all-target strict0; actual App `tool_body` tests3/0/0, runs `wp-timeline-narrow-20261115T180704Z-5c9636a0` and `wp-timeline-state-tests-20261115T182140Z-5c9636a0` | V1 row276 returned two static P2s beyond these tests: equal-byte format cache alias and empty-patch precedence. This is historical returned source; the base13 successor execution is listed above and adoption still awaits its bounded recheck. |
| 5e7c1a14 | View/Kit/App all-target strict rc0, run `wp-timeline-state-tests-20261115T180046Z-5e7c1a14` | Base9 union including glyph inventory and diff metadata cache compiles strictly. Later borrowed-failure signature6729c607 is excluded; App/Kit execution remains queued. |
| 4b591a90 | Run `wp-timeline-state-tests-20261115T165618Z-4b591a90`: App126/0/2 + Kit215/0/0 =341/0/2 | Real scoped pacing, borrowed-body and f4 Markdown-call source; excludes later diff metadata/failure-prose changes and base9 merge. |
| 6d8c0cd8 | View all-target strict run `wp-timeline-narrow-20261115T165011Z-6d8c0cd8` rc0 | Includes About and spoken presentation. Base9 and later app-only kernel/source changes require their own union checks. |
| 6d8c0cd8 | View all-target tests1173/0/0, run `wp-timeline-models-20261115T165014Z-6d8c0cd8` | Includes corrected About registry and all four spoken prompt cases. Later base9/source joins remain separate. |
| 2b806402..99646e29 resident packet | V1 row252 bounded STAGED source/requested narrow PASS | Exact4b8 all-target App strict0; prior7ae GPUI3/0/0, same predicate/test bodies with declaration moves only. Not exact4b8 runtime or full A7/product proof. |
| f4c01f61 Markdown helper | Owner standalone Kit all-target strict0; all17 owned Rust blobs equal actuald068 Kit212/0/0 | Shared helper execution; not A7 host actions, root/image authority, UI or V1 package approval. |

Current owner preparations are separate receiving unions on base13. Clone row
Seat f264918c includes W4's exact clean companion; ordinary strict and populated
resident tests are queued. View-only source2d008494 carries exact membership,
canonical human chunk continuity and the borrowed prompt helper while keeping
baseline search raw until the body renderer switches too. Syntax65c505f7 uses
K's minimal shared adapter and CodeStyles with semantic output metadata; its
strict/runtime are queued. No preparation is advertised as consumer-ready.

Historical failed and canceled runs remain evidence of their exact sources.
The body5c focused pass missed two static findings; current289 corrects both,
with actual retained facade regressions through the same Kit factories, no
public bypass or new product cache. The current all-three-crate ordinary run
completed. Only the bounded review and actual consumer join remain for that
minimal packet; the complete A7 renderer still needs its final gate/captures/V4.

## Required atomic joins

1. **Canonical body for A8:** announced app base9 normal merge, then the minimal
   native body packet's own strict/tests and W4's actual caller. The body-only
   vocabulary is already checked on node9c150c6e; it carries no rich event/field,
   receipt, method activation or native authority. No whole node head enters app.
2. **Rich timeline intake:** W23's checked portable fields/events AND scoped tool
   identity. ProviderTurn includes actual thread+turn, SessionTurn uses its start,
   Record remains orphaned. Raw ToolCallId alone cannot merge late updates, Plan
   outcomes or disclosure identity. App code must not infer the current turn.
   Provenance Message must become a user-shaped received bubble before grouping,
   find and copy; Finished/Errored/PermissionRequired/Closed remain delivery rows.
   Current fixture rendering supplies the shared delivery header/literal body,
   not that complete presentation join. External sender ids are not SessionIds
   and grant no sender-navigation authority.
3. **Receipts:** W6's sole scoped ingress admits one ClientMessageId before preview,
   bytes and transport; same ID joins canonical prompt/queue/command echoes.
   W23's actual runtime outcome contract and Tail cutover land together. Unknown
   retains payload without automatic retry; legacy Delivered never synthesizes it.
4. **Residents:** W12's actual storage/admission/lazy visible body/focus/follow
   conversion, A7's real Clone row Seat, W4's callback mechanics, W2's child ref plus
   accepted projection revision, W1's one checked recursive navigation vote and
   other concrete owner adapters must join once. No default Ready for origins,
   global fallback or copied resource bundle. Network mutation consumes the
   original binding/opaque owner ticket after G4's actual connection guard.
   Pacing keys must use that canonical ResidentToken structurally: legacy Shown
   route/session alone does not fence alias retarget with a reused SessionId.
   W21's future typed normal End must follow actual application of every earlier
   record batch on that same binding before W12 seals ingress. Merely queuing a
   batch in a paced reducer is insufficient; SessionEnded status is not End/ACK.
   The existing owner pump remains the single ordering authority, with no second
   buffer or manager. The scoped fold returns only after synchronous application,
   but the guarded End carrier/flush/install union is not yet implemented.
5. **Request delivery:** W4 retains Answering/Unknown/Unconfirmed across unrelated
   transcript watermarks. Exact owner reads/canonical metadata and settlement/loss
   govern it; resolved records beat stale live data. A7 adds no read registry.
6. **Semantic output and actions:** K supplies the single syntax mapping/resolver;
   B2/W6/W13 supply actual scoped links/images/root authorization. Actions remain
   None until then; no copied palette, opener or selected-route fallback.
   The selectable user-prompt draft stays isolated: the current HTML TextView
   rejects rendered find/reveal ranges and its highlight type has no current-ink
   override. B2 must coordinate one opt-in capability in that same engine. A
   retained mark cache key cannot prove that unsupported ranges were accepted.
7. **Notice/plugin producers:** W18/W23 own the trusted durable notice join under
   ADR0033/0084; the notification body stays level+message with EventRecord.seq
   identity. W16 supplies validated installed plugin surfaces separately. The
   current unavailable card is an explicit fallback, not installed rendering.

## Closing procedure

Use the currently announced app line; preserve whole-gate freeze once started.
Announced app base9 db42d9c5 includes the canonical standalone body vocabulary and
S2 glyph inventory. Later published S2 rows make the preserved prefix100;
A7 appends only Wrench/Eye/MicVocal/Scissors after that prefix;
the exhaustive drawing, license metadata and frozen prefix test remain intact.
Local checks are rustfmt, static size/tokens/diff/import/doc scans and test-module
ordering. Cargo/rustc/boundaries/compilation/tests/gates run only through the build
VM helper. No independent glyph path table or lint suppression is permitted.

After joins and narrow checks, run one final whole native gate, then the owned
fake-fleet capture matrix (EN/KO, dark/light, desktop/phone and medium layout),
inspect every image and submit one V4 package review. Rich-runtime captures need
matching checked node/fake binaries; old shared roots prove only legacy behavior.
No real provider, production mutation, service restart or push is authorized by
this package. Those final checks and user-visible multi-session behavior remain
unverified.


## Resumed verification, base13

The minimal body source192769c4 on base11 executed1515/0/2 across all View,
Kit and App targets (run wp-timeline-state-tests-20261115T190455Z-192769c4),
including retained equal-byte format and empty-patch regressions. Its strict run
returned two lint diagnostics, now corrected in the same shared kernel without
changing behavior/assertions. This is not a current-base13 source pass.

Main fa5c's App/Kit run stopped before tests: merging the accepted resident
producer duplicated four earlier snapshot_busy helpers. The duplicate predicates
were verified pairwise identical and the earlier copies removed, keeping one
canonical definition before the test module. No protection predicate or staging
attribute was weakened. Current base13 narrow/runtime checks remain owed.

## Nested checkpoint ownership

W9's future concrete checkpoint state belongs inside Timeline ownership, not a
new registry or seventh resource vote. Its loading, confirmation, pending and
uncertain states must contribute to A7 readiness before a Ready result is issued;
the actual state moves with the same Area snapshot. Each checkpoint operation
uses a distinct canonical Timeline OperationTicket. W9 must verify its exact
pending binding/ticket and checkpoint/prompt/turn correlation before canonical
consume, then mutate its nested owner only after consume. Full current context
admits local confirmation; queued replies retain original guard/context/binding.
There is no implemented checkpoint state or default Ready placeholder here.

## Published body staging and the full A7 development tree

The announced base14.1 contains the approved8737 body producer with both modules
literal cfg(test). That exact producer and line state remain the consumer handoff.
This separate full A7 development tree already had its own legacy diff delegation
and canonical-header fixture wrapper before that line merge. Those existing
development sources are retained during merge; this does not authorize shipping
their activation relative to the published line. The full package cannot be
handed to the lead until W4's actual caller and legacy delegation join atomically
in a checked receiving union. No rich wire/event/receipt field is activated by
this merge or by ToolDetailSource compatibility methods, which still return None.

Base14.2 adds the canonical pending-permission iterator on the existing waiting
index. The unrelated unactivated identified-message ledger is moved to
pending_messages.rs, retaining its public exported types and tests. There is no
second permission projection or optimistic manager. The incoming SessionResources
constructor owns the shared staged request resource; A7 fixture construction uses
that constructor and retains its reasoning/display setup. All new shared request
modules keep their incoming literal cfg(test) guard.
