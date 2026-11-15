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

Pending narrow tests/clippy: protocol round trips and path validation, and the
staged committed-only resolver. No runtime or parity claim yet.

Next: complete the All private-index case from `checkout-git.ts:3533–3577`
(a path present in the comparison tree but absent from the current index must
not appear as a spurious deletion plus a duplicate untracked addition). The
staged All adapter still uses legacy behaviour and is not complete. Then add
revision/cache/watch ownership, classified history, branch stash metadata,
merge-to/from-base and bounded commit generation before Quality B wiring.
No TypeScript output changes yet: the pending types are not request roots.
Root whole gate and V3 review occur once the node package is complete.
