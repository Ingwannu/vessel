# A8 Requests resource owner prerequisite

Status: source preparation on announced package base 82ca07a7. Exact new-source
strict compilation and runtime tests are owed. No product staging attribute is
removed. This is not the whole A8 package or a response-port handoff.

[Decision Log]
- Purpose: W12 needs one concrete Requests owner per SessionResources, so a
  transport retirement cannot drop a question editor, revive Sending forever,
  or permit a second answer after an uncertain result.
- Existing implementation and constraints: the base already has the sole
  SessionResidents/Residents table, original ConnectionContext, FollowBinding,
  opaque single-use OperationTicket, FieldId, source-aware TextField and G2's
  multiline extension. Its conversation still has one optional Answering and
  one optional permission Sending tuple. It has no A8 response registry,
  multi-question map, failure map or rich presentation/receipt fields.
- Alternatives considered: importing the whole A8 UI branch would import
  unrelated forms, wiring and pending contracts. A second admission table or
  cloned text input would split identity and destroy editor retention.
  Standard Rust borrowing can reserve the existing owner across a synchronous
  mint, so no external library or second state manager is needed.
- Choice: reuse the existing fold's pending indices; add only a shared Request
  identity, small retained-form accessors and a bounded per-resource correlation
  kernel. Its non-Clone Prepared borrows Requests and the actual conversation.
  W12 alone adds the bundle field, factory initialization, readiness vote and
  mandatory retirement/ingress callbacks.
- Why: original node, pending request, actual offered option or actual FieldId,
  unsent form and owner capacity are checked BEFORE the sole canonical table
  allocates a Request ticket. A failed mint drops Prepared without changing
  Sending or drafts. A successful mint is followed immediately by commit,
  without await or intervening state mutation.
- Benefits, costs and impact: no new engine, network call, registry, snapshot
  copy or wire dependency; real fields stay in ConversationArea. The kernel is
  test-only until the coordinated W12 union and is conservative about retained
  forms. It does not prove delivery authority or install a live resident path.

## Module responsibilities and flow

- view::requests::Request keeps PermissionId and QuestionId distinct. The full
  A8 response module must re-export this identity when adopting the producer;
  it must not create a second Request definition.
- Timeline::pending_permissions reads its existing ordered waiting index. No
  second pending map is created; this helper is identical to the A8 source
  exercised in native runtime run wp-side-narrow-20261115T155809Z-730e2db5.
- conversation::requests is literal cfg(test). Requests holds at most 32 exact
  request/binding/ticket correlations and Sending/Uncertain state. Text, drafts,
  caret and editor entities remain in the existing conversation.
- eligibility reads the existing canonical pending body and retained form.
  It accepts only an actual offered permission option, or the matching
  QuestionId plus actual FieldId. FieldId identifies an editor, not a form
  visit, request authority or backend lifetime.
- lifecycle retains uncertainty on exact retirement and releases a retired
  effect only for an exact resolution record admitted through the same
  resident's current original follow ingress. Active tickets still require
  canonical settlement first. Pending snapshots, omission and unrelated
  resolutions cannot release the fence. The hook itself drops no editor.

W12's local-input path first resolves the SAME token plus claim to the full
CURRENT Visible/Focused SessionContext. Inside its single synchronous owner
method it splits the entry and admission borrows, calls prepare with the
original ConnectionContext, mints the sole Request ticket, and commits.
ConnectionContext identifies original provenance; it is not a delivery guard.
Actual ingress must additionally carry the original same-lease DeliveryGuard
and exact FollowBinding. Neither Shown nor a current alias establishes origin.

For a reply, check settling(binding,ticket) before canonical settlement. Only
inside its successful exact settle callback call settled and apply the result
for the returned original Request. Never refresh a network context or retry an
uncertain effect. During retirement W12 invokes retired before replacement
launch, and renderers consult stage(request) to replace Sending with uncertainty.
Canonical exact request resolution reconciles the retained form separately;
merely releasing correlation is not permission to destroy unsaved editor state.

## Scope and provenance

Paseo references remain the permission event cancellation/fan-out and durable
pending-question sections of the feature survey (gaps.md), and the pending card
ownership in question-form-card.tsx. No additional visual parity is claimed.
Rust/GPUI and the existing Vessel field engine remain the implementation stack.
The producer makes no root/proto/client/link/Kit/Cargo/FTL or product renderer
change. Rich ACKs, provider presentations, synthetic plan modes and full A8
Forms/Failures/response Registry are deliberately absent dependencies.

Current full A8 source evidence: 730e2db5 runtime 1492 passed, zero failed,
two existing ignored across View, Kit, Link and App; its successor 6de82f0a is
docs only. The latest strict run at 345e2536 failed nine test lints, all corrected
afterward; there is no current full A8 strict PASS. The new producer's kernel,
baseline accessors and staged tests differ from that runtime source, so those
1492 tests are prior-source evidence, not this packet's execution proof.

## Verification

Use the remote build VM for all Cargo, including boundary checks. Locally only
pinned rustfmt, size, tokens and diff checks are allowed. Requested narrow scope:
strict all-target View/App Clippy and actual App requests_tests GPUI execution.
Tests use the existing private in-memory registry and actual retained TextField;
no provider, fleet, browser, live configuration or service is started.
Final product union, full package gate, captures and V4 review remain owed.

### Staged harness limit

W12 explicitly authorized one literal cfg(test) child registration in
layout/residents/mod.rs and its small requests_owner_tests private helper.
It splits the SAME entries/admission, validates exact binding/entry, prepares
original node and actual form BEFORE mint, then commits without await. The
Requests controller is fixture-local: tests do not prove actual bundle lifetime,
vote collection, guards, wire delivery or product registration. W12 alone owns
those joins. The real retained SessionResources.conversation and its actual
TextField are used; no drop/reprepare sequence is claimed as atomic proof.
