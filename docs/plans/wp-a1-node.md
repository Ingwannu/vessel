# Creation operations and terminal profiles

ADR 0091 records reference behavior, authority, alternatives and compatibility.
ADR 0082 owns workspace identity/provisioning; consume its reviewed contract
instead of deriving identity from cwd or persisting another registry.

## Structure

- `vessel-proto::creation`: validated keys, revisions, phases and typed receipt
  snapshots; create/observe contracts composed with the shared registry types.
- `vessel-proto::terminal_profiles`: bounded profile values and pure literal
  prompt substitution; no process or configuration I/O.
- `vessel-store::creation`: admission/resource claims and receipt transitions
  through the existing SQLite writer/read pool, migration appended over head.
  Reserve count and worst-case retained logical bytes atomically before
  effects; retain every key/claim indefinitely, with typed capacity refusal
  instead of eviction. Existing-key reconciliation and bounded phase growth
  remain possible at capacity. ADR 0091 names the limits and required tests.
- `vessel-node::creation` and `service/creation`: admission, operation actor,
  revision observation and delegation to workspace/session/terminal services.
- Existing terminal/config services: persist profiles with existing protected
  settings writes; launch using existing process/group/environment handling.
- Existing method table: pending rows only after Quality B; no alternate
  dispatch or provisional version number.

## Implementation and evidence

The existing writer/savepoint was extended: creating another database, async
mutex or workspace registry would split admission authority. No external
storage dependency is needed. A new receipt reserves its full 8 KiB logical
budget and all resource claims before the transaction returns. Each bounded
4 KiB row is encoded before replacement; count/byte refusal precedes claims.
Existing-key reconciliation precedes capacity checks, including after restart.
Keys and claims are never removed when runtime resources disappear.

Profile values at 5c39da5f passed six narrow tests and strict proto Clippy;
row125 found no profile-source issue but returned the capacity contract.
Contract revision295281a2 passed row129. The earlier count-only store slice
b4353ae6 passed five real-writer tests; those results do not validate the new
byte accounting. Current byte enforcement, corruption checks, saturation,
concurrent last-slot, bounded growth and full-restart tests are staged for
narrow checks and source review. No receipt service, RPC or launch is approved.

Approved shared identitye45a5d89 is merged. Unreviewed registry producer code
was removed; registry ownership/provisioning remains behind the planned typed
boundary until its producer passes review. Source-created change requests
must carry ADR0082's explicit blocked-automation gate before provisioning.

Next: narrow proto/store checks; receipt source review; service orchestration
with typed registry/runtime fake; pending-method registration; native
integration, package gates and reviews.

Receipt observation uses the existing writer's post-commit effects, mirroring
its coalesced inbox wake-up pattern. Subscribe before the initial SQL read;
each wake-up re-reads the authoritative receipt. Notifications contain no
prompt/argv and retain only one unit value, so intermediate revisions may
coalesce. The writer sends them before acknowledgements; a cancelled caller
cannot suppress notification. A rolled-back savepoint produces no wake-up.
No new RPC or runtime provisioning is registered by this storage addition.
