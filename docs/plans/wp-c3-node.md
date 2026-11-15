# C3 git runtime

Status: staged implementation, no new method admitted. ADR 0087 is the contract.

## Current structure

`vessel-proto::workspace::panel` adds comparison/whitespace/page identity without
changing legacy DiffScope or its admitted calls. `git::diff::scope` stages the
committed-only resolver behind the existing Quality B test boundary. It shares
the protected runner, merge-base resolution and typed summary/file parsers.
No node dispatch, scope table, transport, dependency or permission changes.

Reference: `utils/checkout-git.ts:3508–3530,3580–3609` distinguishes base..HEAD
from base..working files, returns no diff without a base, and falls back to the
resolved base when there is no merge base. Requested bases are never replaced
by a default after a resolution failure. Tests exercise actual isolated git
repositories, including mixed changes, index bytes and unborn HEAD.

## Resume point

The first narrow clippy stopped on documentation markup; the correction passed
protocol/workspace clippy. Library tests passed 382, failed 0, ignored 1.
This proves the staged resolver and contract cases, not node RPC/UI parity.

The staged All private-index case from `checkout-git.ts:3533–3577` is implemented
(a path present in the comparison tree but absent from the current index must
not appear as a spurious deletion plus a duplicate untracked addition). The
shared diff/summary/file readers accept a node-owned optional index; legacy
scopes leave it absent. Restoration uses repeated `--cacheinfo` arguments,
avoiding a stdin runner extension. Path batches bound both count and escaped
argv size; the runner's existing literal-path environment is reused. Scoped
summary and file reads both use the same private index. It reuses the existing
snapshot pattern and test-only tempfile dependency; no manifest/edge change.
Restoration errors retain cleanup errors as well. Explicit blocking-worker
close is covered; active cancellation/shutdown lease ownership remains a
Quality B blocker before this module can leave test staging. New All cases
await narrow checks. Next: merge the announced Quality B node base, then add
revision/cache/watch ownership, classified history, branch stash metadata,
merge-to/from-base and bounded commit generation before Quality B wiring.
No TypeScript output changes yet: the pending types are not request roots.
Root whole gate and V3 review occur once the node package is complete.

The Quality B base 29eebb8b is merged. Its post-merge narrow submit ended with
no execution id or step log; server status lists neither own label as live.
No check result is inferred from that lost submission. The next source also
uses the existing regular-file opener for index copies (no FIFO or final
symlink), an exclusive 0600 destination in the 0700 temporary directory, and
a 64 MiB streaming byte cap. This cap is a stated safety difference from the
reference's unbounded index copy. Tests use a small injected typed copy budget
to prove over-limit rejection and actual modes. Clipped ls-files output or more
than the bounded restoration set is a typed failure, never false completeness.
The next narrow clippy found discarded parse causes in base-index decoding.
UTF-8, object-id and path errors now have separate source-carrying variants;
whole typed-shape and source-chain tests replace the generic error assertion.
