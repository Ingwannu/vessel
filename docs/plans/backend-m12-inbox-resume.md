# Inbox and push: resume

## Resumed node-only scope (2026-11-14)

The current scope is node/wire/store/client/CLI/testkit/generated types and
documentation. Native link/view/screens belong to the app workers; historical
native next steps below are superseded by that ownership change.

The node line `b981f5f2` (protocol 18) is merged for validation. Inbox/push
provisionally use 19, minimum 2; the appended inbox migration is 14, after
search's 13. Changed-crate all-target type checking, 37 store unit tests,
and both TypeScript generating tests passed. Web lint/typecheck/unit/build
passed. The whole root gate passed: 1,814 tests, zero failures, 10 default ignored
measurements, with `RUST_TEST_THREADS=3`; fmt and clippy passed. Full web E2E
also passed all 62 tests. The first root run's missing inbox entries in the
pinned forwardable-method expectation were fixed before this successful run.

The old theme-paint failure is the upstream race fixed in `59000e29`: the
page paints the optimistic setting before its bridge write finishes, while
the old test immediately fetched the served index once. The merged test
polls for the persisted state. This explanation is grounded in
`web/src/data/prefs.ts::preferThrough` and that immutable test correction;
the current E2E theme test passed (1.9 seconds), as did all 62 tests.

Current next work is node acceptance validation: simultaneous answers,
actual CLI question answering, and the oversized-tool update boundary. These
are not claims about delivered screens. See the plan's client-call table.

## Previous stop checkpoint (2026-11-13)

The current slice is the node inbox and account-free Web Push (ADRs 0040,
0041). The node checkpoint's exact commit and final gate results are recorded
below before handoff. Review has not passed; no repository push or deployment
was made.

## Committed and reviewed

- Node checkpoint: `47a4bcb1` (a WIP checkpoint; not merge-ready).
- Resume/contracts checkpoint: `146eee00`.
- Reviewer V1 has interim source findings, not immutable-diff approval.
- Formatting, whole clippy and both TypeScript generating tests passed before
  the final test-expectation correction. Final clippy: passed after the correction (whole workspace/all targets).
- Web lint/typecheck/build passed; 55 unit files / 243 tests passed. Web E2E:
  35 passed, one unresolved failure at `page.spec.ts:34`: the served-index
  theme was expected to be `dark` and was `null`. Its cause is not established.
- The first exact whole test, `RUST_TEST_THREADS=3 cargo test --workspace`,
  stopped at node E2E: 868 passed, one failed, seven ignored across 37 reached
  test binaries; node E2E alone was 144 passed / one failed. All 14 new push
  policy/crypto/delivery/admission tests passed. The failure was the huge-form
  expected field order: all 32 recovered field payloads/options matched
  exactly, but ACP uses a lexical BTreeMap. The expected order was corrected.
  Re-execution after that correction: stopped at the work-window deadline (exit 143); 131 tests passed
  across 6 completed binaries, with no failure observed in those
  completed binaries. It did not reach node E2E or the later store binaries. Later test binaries
  were not reached by the first run; no complete root gate is claimed.
- No native gate, screenshots or phone-host delivery passed. The web fleet's
  recorded processes were verified gone after teardown. The final whole-test
  process tree was stopped by verified PID identities; no live identity or
  executable from this track's private build directory remained.
- The Web Push dependency inventory was independently matched against its
  60 locked transitive packages and their published manifest licences; all
  28 new lock entries are listed in ADR 0041.

## Current contracts

Each node owns its inbox and shared read state; clients fold by node identity.
Read-only devices can read/mark but cannot answer. Agent origins, including
forwarded agents, cannot mark read. Pending forms above the projection pool
budget remain exact references; opt-in `inbox.get` live-request mode recovers
one actor-owned form after history pruning, capped at 15 MiB per reply.

Push uses node-owned VAPID keys and encrypted generic payloads. Its final
admission reselects a current source in the original burst after encryption
and DNS, checking the current device, scope, lease and read state. No real
push-service or phone-host delivery was tested. Windows ACL behavior remains
unverified. Desktop delivery is per window, a documented difference from
Paseo's selected-recipient local transport.

## Next work

1. Finish any unverified node checks and fix actual failures before calling
   the node slice complete. Report the immutable checkpoint and logs to V1,
   then the coordinator/node integrator. Do not treat a WIP checkpoint as a
   passed slice.
2. At a clean point, merge the lead's head; preserve both local Cargo target
   configurations and the supplied font files. Reconcile the appended store
   migration and additive protocol-12 tables; do not change the minimum.
3. Native drafts are backed up outside the checkout, separately from later
   prepared code. They are not a reviewed or compiled API. Restore source
   with current timestamps and verify that changed crates recompile; never
   copy preserved old source times or use cargo clean as a workaround.
4. Complete link/view, then the inbox area. Use C's shared Place
   answer/busy/tool hooks and request cards; never a second form. Keep large
   bodies out of shared head-page fanout. Deadlines must start at enqueue,
   refuse expired queued writes, and settle even while the fleet worker is
   stalled. Keep completed large responses within the four-call budget until
   the UI consumes them.
5. Implement the success-only `inbox_book_edited(&Edit, cx)` hook using
   `view::machine_book::Edit` / `Name`: rename local state, clear forgotten
   state and ignore late replies, make-default is a no-op. Consume cached
   fleet data only from actual Fleet events, so an action reply between a
   book edit and the next Fleet cannot resurrect an old name.
6. Finish pure view fixtures for single-form recovery, aliases of one node,
   failed/uncertain writes, pagination and all translated reasons. Follow
   GOAT DecisionInbox; use C's renderer and `answer_words`.
7. Registration remains a separate minimal commit in the lead's area/route
   files. Report both implementation and registration hashes and validate the
   final combined head; no intermediate-head build claim.
8. Run the entire pinned native gate, the real fake fleet, and inspect every
   required capture in both languages/themes/sizes and empty/loading/failed/
   read-only/long states. Stop only processes started for these checks.

## Other tracks waiting

- S2 awaits a gated native model/area hash for nav/counts, board waiting,
  desktop notification and sound; it must not consume the drafts as passed.
- E2 awaits the success-only book-edit hook hash, called only after durable
  success. A failed edit calls nothing.
- C supplied reusable cards and exact-record reveal; its tool hook removes
  the need for fabricated timeline records. Remove the two reveal dead-code
  expectations at their first real call.
- N3 reserved `GetDeviceSelf {}` -> `DeviceSelf {device, scopes}` and
  `NodeClient::device_self`. Until its reviewed commit is available, one
  access helper stays Unknown and the UI offers the answer and words the
  node's typed refusal. Never infer a grant from an unknown scope response.

All builds use the shared cargo-slot helper, one cargo command at a time.
No custom/prebuilt-only test runner, no real coding agents, no preview node,
no push or release. The precise private artifact paths and process records
are in the local resume note.
