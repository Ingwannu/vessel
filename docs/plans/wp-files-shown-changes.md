# Shown Changes provenance

Status: isolated source preparation on the exact Devices delivery producer.
Its shared delivery exports remain owned by that producer; this package neither
re-promotes them nor claims a checked product handoff before its green line.

[Decision Log]
- Purpose: git actions must identify the original connection that supplied the
  summary or diff actually drawn. A root receipt alone cannot identify it.
- Existing assets: one CodePanel owns Changes.summary and per-path diff states;
  session_root resolves the physical root; G4 owns actor admission, Unary calls,
  bounded guarded egress and the opaque DeliveryGuard.
- Alternatives: stamp legacy results with the current Machine (unsafe after
  rebind), duplicate the Changes controller, or infer roots from cwd (both lose
  the original acquisition boundary).
- Choice: replace the existing panel root acquisition with an original-context
  GetSession request, then send its GetChanges/GetFileDiff through the same
  selected connection. Keep only correlation metadata alongside existing data.
  Each result retains the exact actor-minted guard, original context, root and
  monotonic owner ticket. Guard validation occurs at installation and every
  currentness/getter lookup, including before queued Fleet retirement arrives.
- Why: delayed results cannot acquire a new connection's identity or survive
  irreversible retirement merely because their displayed counts match.
- Consequences: refresh, root/session/comparison changes and superseding requests
  invalidate write admission immediately. Old rows can remain read-only while
  loading. Untagged legacy data never yields an admitted shown-source getter.
  Workspace-root projection stays unavailable pending its canonical runtime
  producer. The stamp is source evidence, not permission, UI-visit identity,
  mutation approval, a replacement for action tickets, or a second resolver.

## Structure and data flow

The existing view FileScope and FileTickets values retain request facts and
non-reused correlation. Link's workspace code-source module routes only existing
GetSession/GetChanges/GetFileDiff, with named readonly deadline and admission
bounds. It uses selected_connection once, then selected.call and send_guarded;
no raw client or public guard constructor is exposed. Queue refusal settles the
same local request and never installs a Ready stamp.

CodePanel keeps the one root acquisition; Changes adds provenance metadata to
its existing summary and diff states. A new summary invalidates all diff origin
receipts. A paged diff retains its parent summary and prior page identity;
late/superseded pages cannot append into another comparison's body.

The opaque shown summary/diff context exposes original connection/root/scope
facts and is checked against the current owner before use. Legacy host lookup
requires the shown Session/Changes host. A typed Explorer lookup additionally
checks its real PanelContext; it does not fabricate a tab or Workspace root.
G1 owns the subsequent action intent, source digest, approval and preflight.

## Validation

Required regressions: queued success followed by retirement before installation;
retirement after installation before public lookup; refresh and A-B-A comparison
supersession; wrong root/session/ticket/parent page; readonly legacy results;
reconnect under the same alias never re-stamps old data. Actor evidence must use
the shared real framed fixture. No fabricated DeliveryGuard, sleeping race, real
agent, browser, or service operation is part of these tests. All Rust execution
runs through the build VM. Source preparation alone is not execution evidence.


The current getter/accessor entry points remain literally test-only pending the
real G1 consumer, so this preparation cannot turn an alias-only toolbar into an
admitted writer. G1 must capture the opaque source while drawing its actual
action and check that exact source at callback time; looking up a new source
for an old click would erase the UI-intent boundary. Root/session/path response
identity is validated before installation. Diff continuation uses the existing
DiffBody next-page value, with the original parent summary and page ticket; no
second diff body or snapshot store is introduced. This proves correlation, not
an atomic repository snapshot across requests.

The isolated preparation currently has seven authored real framed App cases;
they have not run. No G4 export, selected-client method or delivery-module staging
attribute differs from its exact producer. A typed Explorer getter is still
unavailable until the real mount/current-target join is included; no synthetic
Explorer tab or Workspace root is projected here.

The delegated retirement helper is test-only and registers no Git owner. G1's
single actual hook can invoke it with the original Notice; the fixture exercises
that delegation through the real layout transaction. A typed getter uses the
existing exact-target and visible-panel queries and explicitly excludes logical
Workspace projection. While Explorer is unmounted it remains None. Switching
legacy tabs or closing the host invalidates stamp metadata before another read;
it never aborts the detached readonly request or replays a write.
