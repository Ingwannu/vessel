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

The isolated preparation has eight real framed App cases. They executed on
340f92ede in wp-files-shown-20261116T012821Z-340f92ed: 8 passed, 0 failed,
0 ignored, 143 filtered. Fonts and Cargo steps both returned zero. This is
scoped source-acquisition/retirement evidence, not a whole-package gate. No G4 export, selected-client method or delivery-module staging
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


## Reference and intentional boundary

Paseo's `packages/app/src/git/use-working-diff.ts:16-24,51-85` keys the
comparison and review draft by server/workspace/cwd/comparison. Its
`git/use-diff-query.ts:44-97` includes comparison/query scope and retained-panel
activity in the read route; `git/diff-pane.tsx:1551-1583` draws the returned files
and publishes that same review attachment. Vessel reuses its existing typed
root, summary and paged-diff controller rather than introducing another replica.
The added irreversible admitted-epoch guard is a deliberate Vessel boundary: an
alias, equal counts or a cwd cannot authorize re-stamping already drawn data.


Before the first queued preparation check acquired a slot, a static pass found
an unused RetargetRequest result and needless ownership in the root/diff
installation helpers. The queued run was verified and cancelled terminal143
before one replacement. The helpers now borrow their original requests and
the closed Link Answer exposes only read-only request facts, currentness and
consuming decomposition; callers cannot manufacture a successful envelope by
attaching a guard. The actual guard still comes only from G4's selected client.
No original assertion, correlation fence or G4 export was changed.


A second static pass found that the getter impl also needed the literal test
attribute matching its test-only Shell import. The impl and opaque workspace
type facade are now consistently staged. The source tests consume that facade
with explicit types, so the eventual G1 adapter can store the opaque snapshot
without exposing constructors or private code-module paths. A missed doc type
backtick is corrected in the same batch. The still-queued preparation check is
replaced once after terminal cancellation; no executed result is inferred.


[Decision Log]
- Purpose: the readonly source producer must not trigger the legacy editor's
  alias-only flush when an epoch/session transition is detected.
- Constraint: the shared line does not yet contain C5's retained file-owner
  cutover. Its legacy root-follow path flushed and discarded the viewer.
- Alternatives: copy the entire unverified editor owner, or cause a new legacy
  write as a side effect of acquiring readonly summary provenance.
- Choice: while any legacy file viewer remains, invalidate summary admission
  and refuse the root-follow transition before flush or viewer removal. The
  existing Back/close path must release it before a new root can be acquired.
- Why: this preserves actual engine text and original pending payload without
  a second owner, guessed write settlement or automatic retry.
- Impact: this portable producer is deliberately conservative, not complete
  file-navigation parity. The real retained-file adapter replaces this hold
  when it joins the shared line. No general legacy autosave safety is claimed.

## First executed compilation

Run wp-files-shown-20261115T173821Z-75762e0f reached the real App target
and failed before runtime: the replaced session-read helper was narrower than
its existing workspace caller, test envelopes used request-type names instead
of canonical GitChanges/GitDiff enum variants, and one fixture used FromStr
where HostAlias exposes parse. The correction restores only the existing
caller visibility and uses canonical enum/construction APIs; every correlation
assertion remains. This failed run is retained. The newer base10 union and
correction still require compilation and actual framed regression execution.

The next actual strict run reached App and returned ordinary lint findings:
explicit imports, unit-arm semicolon, two private-owner visibility bounds and
a consumed getter result. The test module used a combined cfg(all(test,...))
that the strict test-context lint does not recognize for helper assertions.
It now uses the same separate cfg(test) and native-target attributes as the
shared actor fixture. No production lint allowance, assertion removal or
transport-policy change is introduced. Actual fixture assertions remain test
code; the normal product build still excludes them. This correction is on
the announced base11 union; exact new checks remain pending.

## Executed preparation and adoption boundary

The prior exact 1ef7dd28 preparation passed View/Link/App all-target locked
Clippy with warnings denied (wp-files-shown-20261116T002923Z-1ef7dd28).
The 340f92ede merge adds announced base13.1 without changing this producer's
owned Rust. The eight framed cases above then passed on that exact merged head.
The earlier queued first test run was verified queued, cancelled terminal143
and replaced once under coordinator approval; cancellation is not test evidence.

The executed preparation also contains G4's separate Scenario and Devices
producer. Those changes are not C5-owned and are not part of a whole-branch
adoption request. The canonical delivery guard on the shared base is reused
without changing its exports or adding constructors. G4 still owns qualification
and landing of its fixture/product successor. A consumer must preserve that
dependency boundary or check its actual receiving assembly. Scenario cleanup
drains its Devices workers; these cases do not prove a generic C5 worker drain.
The opaque shown-source consumer accessors remain staged until a real G1 caller
joins, with its own action admission and changed-source review.

## Bounded review verdict

V1 independently accepted the C5-owned source and requested scoped narrow
checks on exact340f92ede and docs5714f87f7. All22 owned Rust blobs match the
prior strict source; raw checkout, log hashes and eight case names were checked.
The nine Scenario/helper/native-RPC manifest paths match the separately reviewed
G4 final51f64421 producer. This is a conditional source/evidence result.

The prerequisite and receiving assembly remain on HOLD: wait for the lead's
announced G4 green line, form a straight C5-only producer there, and check that
receiving assembly. Never merge the whole preparation or promote partial G4
exports. The page case covers parent invalidation rather than full multi-page
runtime. These tests invoke owner handlers over a real framed Scenario; they do
not exercise the private product event dispatch, painted UI or OS input. The
stamp supplies no UI-visit ABA protection, mutation approval or compare-and-swap.
G1 received the exact hash and this verdict; its real caller must preserve its
own action/visibility lifetimes and original preflight. No unchanged rerun was
requested by the reviewer.
