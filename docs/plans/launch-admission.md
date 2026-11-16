# Shared live launch admission

Status: proposed implementation; checks and review owed.

[Decision Log]
- Purpose and intent: a preparer held across node stop must not start a new
  process while existing actors remain alive for orderly settlement.
- Existing assets: Runtime already owns final actor shutdown and Node already
  owns its early stop token. Reuse those sources and the existing spawn paths.
- Alternatives: a second stop token or preparer-local predicate would drift;
  closing actors first loses the required settlement order.
- Choice: one cloneable read-only observer, one live predicate and cancellation
  observation. It supplies liveness only, never caller authority or a permit.
- Cost: check/spawn is not atomic. Existing cleanup remains for a start racing
  the final check. Held physical preparation and caller/policy freshness remain
  independent obligations of their own consumers.

The existing runtime factory moves unchanged to node/setup before extension,
so the owning node file's size allowance shrinks. No settings writer, disk/SQL
model, recovery coordinator, Journal or restart runtime joins this prerequisite.

Paseo packages/server/src/server/daemon-config-store.ts:655-659 serializes
settings operations. The explicit Rust early-stop/final-runtime distinction
and this shared pre-spawn observer are implementation choices for that
ownership order, not claims that Paseo has the same API.

Resume point: factory relocation is source-only; admission implementation,
actual held-lookup regressions, remote strict/runtime checks and review follow.
