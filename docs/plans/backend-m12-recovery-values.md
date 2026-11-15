# Startup recovery R1: values and settings reply projection

R1 starts on the node line, independently of the unmerged recovery runtime.
ADR 0059 records the reference, defaults and full recovery contract. This
slice implements only the values and peer projection needed before wiring.

[Decision Log]
- Purpose: recovery values must have one validator/default and remain hidden
  until their client and admission boundaries can preserve them.
- Existing parts: PromptText owns validation and the byte cap; Reply's existing
  for_protocol walker already handles nested fleet serialization.
- Alternatives: a second prompt validator, a locally assigned protocol, or
  request registration before the shared accessor would duplicate or bypass
  existing boundaries. No external package is needed.
- Choice: extend those value/projection APIs. One Option protocol constant
  remains None, and both settings replies use the same projection.
- Cost: NodeSettings loses Copy because it now owns a bounded prompt. Its
  existing builder gains only the disabled default field; no engine is added.

## Scope and review sequence

R1: policy and partial-patch values; reused PromptText bound; default/debug/
verbatim/multibyte tests; old/current-unassigned/synthetic-assigned projection;
Get/Set and Forward/Relay/Deliver/nested tests; generated TypeScript.
No request flag, request patch field, Call::since change, new CLI setting,
engine, runtime, store migration, assigned protocol, page or native implementation.
The existing CLI settings renderer borrows its input after NodeSettings loses
Copy; its existing command, output and requests are unchanged.

R2: existing settings/file engine and recovery service staged under cfg(test)
as A10 requires, reviewed as source reviewed, staged. A raw legacy Admin setter
regression seeds a nondefault policy through DataDir and checks actual reply
bytes. No new-field RPC or test-only dispatch/authority bypass is permitted.

Wiring: after the shared required_protocol accessor and single method table
are on the node head, consume them, register fields, connect the clients/CLI,
remove the staging attribute and run/review the whole changed product tree.
The older unmerged runtime remains a frozen reference, not a merge candidate.

## Base and preliminary verification

The candidate was first prepared on node `0618c4fb`, then reapplied on
`dabf9888` (protocol 23). Only README and ARCHITECTURE conflicted: their
complete new base text was kept, with R1's status paragraphs appended. No
migration, method table, request type or existing assertion was rewritten.
That newer base was not an announced green head: its node Clippy failed on
an unchanged limits test's function budget (101/100). The integration line
already repaired it. With the coordinator's explicit permission, R1 moved to
`e9c2e32a`; that fast-forward changed only the inherited limits test and had
no conflict. Its integration gate was still pending when this candidate was
prepared. A failed base gate cannot be presented as R1 verification.

The initial new multibyte fixture used integer division; it now uses checked
quotient/remainder from the existing byte cap. A later new iterator predicate
was simplified to the equivalent Option::is_none_or expression. Neither
change weakened the bound or assertion. Narrow proto all-target Clippy on
the protocol-23 candidate passed (56.88s). The inherited node lint failure
above remains recorded separately. Earlier 187/0 proto tests and node Clippy
on `0618c4fb` are preliminary, not evidence for the new base.

Generated TypeScript was produced by the repository's generator: both proto
and bridge freshness tests passed (2/0 after a 4m37s build). Its only declaration
changes are NodeSettings' optional response field and the RestartRecovery
value. The partial patch is not reachable from a registered input. Generated
whitespace is retained verbatim. Format, file-size and dependency-boundary
checks passed before candidate submission; full verification is still owed.

## Gate transport and remaining evidence

The whole gate uses the shared remote runner on this immutable candidate,
as required by the build policy. The candidate is committed before submission
because the runner transfers Git objects only; this does not claim a passing
gate. A narrow node check precedes the whole run. The run id, exact commit,
per-step return codes, load and totals are retained for review. The previous
comparison repair's exception remains separate.

R1 currently makes no claim about startup execution, raw saved-policy setter
bytes, native/page rendering or another operating system. The actual legacy
Admin setter regression belongs to R2's staged engine. NodeSettings now owns
a prompt and is Clone rather than Copy: consuming native settings views must
borrow their saved result, and const settings fixtures must become factories.
Those mechanical consumer changes belong to the native owner.

## First whole candidate

Candidate `de8b6a78` ran as `sessionextras-20261115T015726Z-de8b6a78`.
Web preparation, format, size and boundaries passed; workspace Clippy stopped
on the existing CLI renderer's now-non-Copy settings argument. The original
run remains failed (Clippy 101; run 1), with load 15.54 at start and 14.02 at
end. No tests or later stages ran. The renderer and its sole caller now borrow
that value; there is no new CLI setting, output, request or behavior. A new
narrow check and changed-tree whole run are required for this correction.

## Announced-base port after the failed newer-base run

The second candidate `0edb4bfe` ran as
`sessionextras-20261115T021502Z-0edb4bfe`: the workspace had 2,075 passes,
three assertion failures and 12 existing ignored tests. All other 17 steps
returned zero: TypeScript freshness (2 tests, no generated diff), all Clippy
variants, the built page and binaries, web 417, browser E2E 62 and forge 4.
Load was 8.67 at start and 8.29 at end. The whole gate remains failed.

The failures were unchanged inbox/push tests comparing their protocol 22 to
the newer global protocol 23, and the unchanged forwardable-method vector's
order after integrating limits. No method requirement, scope, table, global
version or any of those tests was changed by R1. These are assertion failures,
so no timeout exception or unchanged rerun applies. They are retained as
failed evidence, not a passing implementation gate.

R1 needs no protocol-23 feature. Under the coordinator's explicit base choice,
only R1's own 14-path delta was ported to the announced green node head
`5ac9c991` in a new isolated branch. The failed newer-base candidates remain
unchanged in their original branch. Only README and ARCHITECTURE conflicted:
the complete announced-head text was retained and R1's sections appended.
Every code and generated declaration hunk applied without conflict. No limits
code, failed-test repair or other newer-head feature was brought along.
The current global protocol stays 22; the recovery feature remains None, with
no migration. New narrow checks and a whole gate are owed on this announced
base before submitting a gated R1 slice. R2 and wiring remain pending.
