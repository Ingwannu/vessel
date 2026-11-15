# Timeline reference provenance across fleet hops (ADR 0084 amendment)

Status: shared wire/security proposal following the integrator's 10:1x ruling;
V3 must review this boundary before its runtime wiring. The previous stateless
namespace codec alone is insufficient: a second outgoing adapter cannot know
whether a prefixed id is an original native spelling or a previous projection.
No feature number or second constant is assigned here.

## Concrete shared wire shape

The marker belongs only to the existing node-to-node reply carrier:

```rust
pub enum TimelineReferenceOrigin {
    LegacyProjected,
}

pub struct Forwarded {
    pub reply: Box<Reply>,
    // None means canonical only on a connection supporting TIMELINE_PARITY.
    pub reference_origin: Option<TimelineReferenceOrigin>,
}
```

It is read-only reply metadata, not a client request parameter or a tool id
prefix. `None` is omitted. `Some(LegacyProjected)` can only reduce trust: it
cannot assert that a legacy reference is canonical, select a permission option,
or name an authenticated caller. The field is carried between modern fleet
peers only when the same `TIMELINE_PARITY` gate is negotiated. No new constant,
method, schema table or authority is introduced.

## Ingress and egress ownership

1. The target keeps complete original ids/scopes in its durable records. Every
   connection retains the existing `for_protocol` masking edge.
2. `Links` captures the actual connected client's negotiated protocol together
   with the `Forwarded` response to `Deliver` / `RelayCall`. If that connection
   lacks `TIMELINE_PARITY` (including an unassigned gate), it marks the whole
   inner reply's tool references LegacyProjected. This ignores any sender claim
   of canonical origin; the connection is the evidence. Other generic node
   calls do not acquire this marker by payload spelling.
3. A modern peer retains an already-LegacyProjected marker received from a
   modern fleet connection. A canonical reply remains unprojected between peers
   supporting the feature. Native provider codecs never construct or accept the
   marker. Ordinary client calls have no accepted origin input.
4. Outgoing recursive reply adaptation carries a typed reference context into
   the inner reply. In canonical context, the existing injective tagged codec
   projects scoped identities and escapes original native ids in its reserved
   namespace. In legacy context, all ids remain opaque and unchanged: no second
   projection and no scope recovery by parsing the id. Other required masking
   (new variants, attachments, optional presentation, and so on) still runs.
5. When the next peer is old, its marker is removed after applying that context.
   An older intermediate may drop the unknown metadata; the next modern node
   derives legacy origin again from that older connection. There is no refusal
   merely because a marker was dropped.
6. All references under a legacy context are display-only. A resolver requesting
   a scoped mutation from a legacy tool reference returns the existing typed
   Unsupported/protocol error. It must not resolve a ToolCallId to a canonical
   tool or permission by spelling. Existing node-assigned PermissionId authority
   remains distinct; the marker never substitutes for it or for option ownership.
7. There is no special client-header scan or second parse path. The marker is
   only a reply field; no client request type reads it. A client-sent unknown
   `reference_origin` key is inert and cannot construct provenance. If a header
   type later chooses explicit unknown-field refusal, its ordinary typed decode
   reports through the existing DecodeFailure privacy path.
8. Unknown origin spellings from a newer peer decode conservatively as
   LegacyProjected (serde's other arm), never as canonical or as new authority.
9. While TIMELINE_PARITY is None, every forwarded reply's tool references are
   legacy. The new scoped-reference mutation resolver therefore refuses across
   fleet hops during staging. Existing permission answers use PermissionId, not
   tool-id resolution; today's forward/relay flows must keep their original tests.
10. Typed reference context is applied inside one existing for_protocol traversal.
    Each recursive Forwarded/Relay/Deliver reply is visited once, without a
    pre-pass or a second rewriting pass.

Fleet subscriptions are not forwarded by these envelopes: existing Subscribe /
Unsubscribe machinery remains separate. Its direct owned/borrowed event encoder
must continue to use the same single outgoing mask; direct legacy live output
must match a legacy event page obtained through the fleet chain. Any future
fleet event proxy must carry equivalent connection-derived provenance rather
than reuse an unclassified EventRecord as canonical input.

## Required evidence before integration

- Canonical A/B same raw id, explicit late A update: a legacy raw-id fold keeps B
  unchanged, and permission B references its own projected announcement.
- Original native id equal to a valid-looking reserved projection: it remains
  distinct after modern -> old -> modern decode/re-encode.
- Repeated projection of already-marked announcements, updates and permission
  references is stable; no l-tag nesting or scope upgrade occurs.
- Real serialized/deserialized two-hop Forwarded / Relay / Deliver replies keep
  ids, record sequence numbers, timestamps and page cursors stable.
- Modern -> modern keeps original canonical scope/id; a received legacy marker
  survives a modern hop and only downgrades trust.
- Client-supplied marker keys never construct provenance or enable dispatch;
  opaque user JSON containing the same key is ordinary content. Unknown future
  reply origins only downgrade to legacy. DecodeFailure remains the sole reporter.
- Scoped mutation resolution from a legacy tool reference refuses. No id-prefix
  recognition or permission-label matching grants authority.
- Existing head tests remain unchanged; actual merged-head strict checks,
  transport tests and the one final whole gate remain owed.

## Paseo comparison and rationale

Paseo's in-memory timeline projection uses provider/turn/call identity and epochs;
Vessel keeps its existing durable EventRecord/EventSeq cursor model. The explicit
legacy origin is a deliberate fleet compatibility floor beyond Paseo's single
projection: a route through an older node must not turn display data into a scoped
mutation target or apply a late update to different work. Connection provenance
solves the ambiguity a stateless text codec cannot solve. This marker is transport
metadata and is never a user-visible execution choice.
