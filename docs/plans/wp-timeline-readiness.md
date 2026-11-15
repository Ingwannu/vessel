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
| Diffs | `view/src/timeline/diff/`, `kit/src/output{.rs,/}`, `tool_body.rs` | Existing virtual output/copy with bounded line and intraline changes. K's single syntax resolver is still absent. |
| Plan and tasks | `plan_card.rs`, `plan.rs`, `view/src/timeline/plan*` | Shared Markdown card; W4 owns pending approval actions. Stable task id/active_form await W23. |
| Turn/tool/reasoning disclosure | `view/src/timeline/{turns,runs,disclosures}.rs`, conversation `{turns,disclosures,tool_sheet}.rs` | Consumed by the existing Chat preferences and rows; full followed log remains available to W1. |
| Footer and context | `turn_footer.rs`, conversation `{footer,meter}.rs` | Known-range duration, failure suppression and full reply copy; exact context danger boundary is >90 percent. W5 owns account limits and additive usage children. |
| New notice/compaction/plugin rows | `view/src/timeline/{items,compaction}.rs`, conversation `notices.rs` | Typed projection/renderer preparation. No activated rich AgentEvent intake. Plugin fallback names plugin/kind and never interprets opaque data. |
| Mounted text reveal | `view/src/timeline/text_reveal.rs`, conversation `pacing.rs` | Product assistant/open-reasoning prefix, first-sight hydration, grapheme safety, reduced-motion/find bypass and weak bounded frame wake. Source-scoping regression executed at4b591a90; later changes remain separately bounded. |
| Spoken prompt display | `prompt_presentation.rs`, `searchable.rs`, conversation `{bubbles,timeline}.rs` | Product source uses one borrowed display slice for user bubble, local search, copy and rewind text. View strict/tests executed at6d8c0cd8; canonical body is untouched. |
| Submission ledger | `view/src/timeline/pending.rs` | Pure cap32 typed preparation only. Legacy Tail is still the sole product optimistic path. |
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
| Portable body5c9636a0 | Exact View/Kit/App all-target strict0; actual App `tool_body` tests3/0/0, runs `wp-timeline-narrow-20261115T180704Z-5c9636a0` and `wp-timeline-state-tests-20261115T182140Z-5c9636a0` | Proposed Ready/Loading draw, structural IDs and diff-cache work are exercised. This filter does not replace full three-crate ordinary tests; source review and final adoption remain held. |
| 5e7c1a14 | View/Kit/App all-target strict rc0, run `wp-timeline-state-tests-20261115T180046Z-5e7c1a14` | Base9 union including glyph inventory and diff metadata cache compiles strictly. Later borrowed-failure signature6729c607 is excluded; App/Kit execution remains queued. |
| 4b591a90 | Run `wp-timeline-state-tests-20261115T165618Z-4b591a90`: App126/0/2 + Kit215/0/0 =341/0/2 | Real scoped pacing, borrowed-body and f4 Markdown-call source; excludes later diff metadata/failure-prose changes and base9 merge. |
| 6d8c0cd8 | View all-target strict run `wp-timeline-narrow-20261115T165011Z-6d8c0cd8` rc0 | Includes About and spoken presentation. Base9 and later app-only kernel/source changes require their own union checks. |
| 6d8c0cd8 | View all-target tests1173/0/0, run `wp-timeline-models-20261115T165014Z-6d8c0cd8` | Includes corrected About registry and all four spoken prompt cases. Later base9/source joins remain separate. |
| 2b806402..99646e29 resident packet | V1 row252 bounded STAGED source/requested narrow PASS | Exact4b8 all-target App strict0; prior7ae GPUI3/0/0, same predicate/test bodies with declaration moves only. Not exact4b8 runtime or full A7/product proof. |
| f4c01f61 Markdown helper | Owner standalone Kit all-target strict0; all17 owned Rust blobs equal actuald068 Kit212/0/0 | Shared helper execution; not A7 host actions, root/image authority, UI or V1 package approval. |

Current in-progress feature execution is corrected base9 App/Kit tests
on5e7c1a14; its all-target strict finished successfully. Main3a strict returned own import/test idiom diagnostics, now
corrected without suppressions or weakened assertions; its red log remains.
Portable body strictf839/ae256 stopped before tests on omitted closure adapters;
corrected5c9636a0 strict and the three focused tests now pass. The same source's
full three-crate ordinary tests are submitted; no whole native pass is inferred.
An older still-queued View74 run was explicitly superseded, confirmed terminal143
before its one ordinary replacement; its retained logs are not a failure/PASS.

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
S2 glyph inventory. A7 appends only Wrench/Eye/MicVocal/Scissors after the fixed95;
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
