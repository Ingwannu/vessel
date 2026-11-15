# A10: named launch profiles, node/client package

## Scope and status

Specification: the Paseo 0.15.4 survey's Host agent profiles, Applying
profiles to drafts/existing agents, and Legacy favorites migration
sections, checked against the original files named in ADR 0076.

The initial CRUD contract passed review. The expanded contract adds live
application and migration and is proposed for review before that code.
Core value source `9375732b` has TS-enabled proto Clippy and 275/0 library
tests, including nine new value cases. That is a checked partial stage,
not this package's skeleton, completion, product review or whole gate.
Results are in `backend-m12-launch-profiles.md` and checkpoint `82776280`.
The original failed Clippy log remains retained.

No call, migration, catalogue, service, consumer, live apply, legacy helper
or app screen is built. The feature protocol remains None. The integrator
allocated the store schema after title origin; consume the announced
baseline before appending its single predecessor-relative constant.
The pending Quality B row is still a registration prerequisite.

## Existing owners and structure

| Owner | Reuse/extension |
| --- | --- |
| `vessel-proto::launch_profiles` | Validated core values, catalogue descriptors, typed state/outcome/notice/legacy inputs; request/reply shapes only when table registration is admitted |
| `vessel-store` | One ordered-list row and revision through its current reader, per-operation savepoint and commit-before-answer writer; no second file/cache |
| `vessel-node` | Scope/origin/ownership checks, finite metadata/probe/apply admission, retained account snapshot and routing |
| `vessel-session` | Existing wake/move/ended/terminal boundary and one captured process input handle |
| `vessel-agent` common input boundary | Shared bounded owner for all single and bundled configuration mutations; actual driver completion, partial prefix, notice and process closure |
| `vessel-client` | Typed calls, pure draft/live materialization and migration planning; ordinary host CAS, no local credential/preference access |
| CLI / MCP / testkit | Same typed calls and fixtures; Read-agent list notes, owned-session apply rules, no real agent prompt |
| App preference owner / G3 | Validated local compatibility input, per-node marker/in-flight state, remembered concrete choices and screens; separate app package |

The shared agent/session boundary needs explicit ownership coordination;
no driver parser or account policy is silently taken over from its owner.
No client-loop approximation of server-side mutation serialization.

## Skeleton states, then implementation

The node skeleton consists of compiled typed state/result definitions and
staged private service/owner seams, tested through the actual owners. It
is not a visible control returning a placeholder success. Proposed states:

- Profile read: unassigned/unsupported, unread, unavailable, complete empty
  or ordered snapshot. Editing: loading, draft, saving, stale revision,
  invalid values, retryable storage failure, committed current state.
- Catalogue: no request, resolving, resolved, explicit unavailable/error,
  configuration changed. Stale selection results never replace a newer key.
- Live preparation: awaiting live options, inapplicable provider/account,
  ready ordered choices with omitted stale modes. Creation remains strict.
- Apply owner: queued, running one actual step, completed, first refusal
  with applied prefix, acknowledgement unknown while fenced, process closed.
  A notice does not short-circuit following steps. No rollback claim.
- Migration: unsupported, marker complete, no favorites, resolving catalogue,
  ready append-only plan, applying, retryable/conflict/unresolved failure,
  complete. Marker storage is app-owned and follows host success.

After expanded contract review and ownership coordination, implement the
state/type skeleton, then storage/probe, shared mutation owner and service,
then table/accessor/client/CLI/MCP/fixture wiring after Quality B. Use one
commit per sub-feature with narrow checks. No public/dead-code/lint escape
for a staged service. The whole root gate and review occur once at the end
of the complete package; no whole gate per type/contract commit.

## Reference behavior and differences to keep explicit

Keep stable ids, exact whole-list replacement/clears/reorder, public notes,
registry glyph/color fallback, host-wide catalogue/defaults, delayed/stale
request isolation, copy-to-draft with no live id binding, immutable live
provider/account, model/mode/thinking/features order, first notice and
partial failure with no mutation interleaving. Invalid saved mode is
omitted only for live profile reconciliation; a deliberate create/explicit
option request is still validated by its normal driver rule.

Keep legacy per-host in-flight/complete behavior, pair/id/selection dedup,
existing-first append, catalogue label/disambiguation/suffix naming, bounded
loading retries and marker only after success. There is currently no
Vessel favorite schema to reinterpret: existing remembered choices are not
favorites. The app owns compatibility data and markers; the node cannot
scan another application's home. UUID migration identity, strict bounds,
CAS reconciliation and typed uncertainty are the recorded differences.

## Verification and handoff

Tests must fail without each invariant: private values/escaped byte cap,
maximum revision, strict invalid later record; two writers/no-op/stale and
failed transaction/restart; scopes and delegated/agent origins; real fake
ACP/direct probe choices, account reload, cancellation and cleanup; shared
single/bundle mutation fence, first error/prefix, first notice, hung reply,
process replacement and retained external handles; source draft/live and
legacy migration cases, no-favorites/no-catalog, failure/retry/dedup/marker
ordering and virtual backoff. Safety waits use within and named phases;
written executable fixtures use the shared executable helper.

Pending-feature refusal is distinct from assigned happy-wire evidence.
No placeholder version is invented for a test. Every old/direct/nested
transport path, generated TypeScript, actual CLI/MCP and fake-fleet calls
are verified at package close. Report that gate by exact run/commit and
all return codes/totals, and list anything only a real provider/OS can
prove. G3 receives only the reviewed, assigned exact API after integration.
No native/page screen, real account/agent, production service, preview
pairing, push or permission mutation belongs to this node package.
