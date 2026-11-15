# C3 git runtime

Status: staged implementation. No new method has an assigned protocol number.
ADR 0087 defines the contract; the Quality B base 29eebb8b is merged.

## Structure and decisions

`vessel-proto::workspace::panel` defines comparison/whitespace/page identity
without changing legacy DiffScope or its admitted calls. `git::diff::scope`
stages committed-only resolution; `git::diff::all_index` stages All through a
private index. Both reuse the protected runner and typed summary/file parsers.
The shared readers accept a node-owned optional index; legacy scopes leave it
absent. File explorer state and workspace registry identity are separate.

Reference: `utils/checkout-git.ts:3508–3609` distinguishes base..HEAD from
base..working files and returns no comparison without a base. All restores
untracked paths present in the comparison tree into a private copy of the
current index, avoiding a false deletion plus a second added entry. Summary
and file reads use the same private index. Repeated cacheinfo arguments avoid
extending the runner with stdin; its literal-path environment already protects
path arguments. Requested invalid bases never fall back to the default.

The scratch parent is supplied by the node and must be absolute. The library
must not read environment/home to choose one. It reuses the existing regular
file opener (no final symlink or FIFO), an exclusive 0600 destination in a
0700 private directory, and a 64 MiB streaming copy budget. This budget is an
explicit safety difference from the reference's unbounded index copy. Argv is
bounded by 128 paths and 16 KiB of conservatively escaped arguments. Clipped
ls-files output or an incomplete restoration set is a typed failure, never
false completeness. UTF-8/object/path decoding preserves its original source.

Current service staging uses the existing test-only tempfile dependency. Before
normal admission, resource ownership must include cancellation/shutdown cleanup,
and any dependency promotion needs the matching ADR/architecture update. There
is no production tempfile dependency, permission change or new transport yet.

## Verification boundaries

- Initial contract and committed resolver at 3611a6b4: protocol/workspace narrow
  clippy green; library tests 382 passed, 0 failed, 1 ignored.
- All/private-index additions have not passed tests yet. At 77ef00b4 narrow
  clippy returned discarded parse causes; corrected at 3353c2d4 with typed
  source variants and source-chain tests. That check remains queued.
- The later explicit scratch-parent change is not included in that queued
  source. It requires its own changed-source narrow check.
- Post-base submissions of 4331b5b2/ea58824d ended before registration during a
  build-helper lock fault. They have no step evidence and were not rerun as
  successes. Registered earlier red/green runs remain unchanged.
- No RPC, live node, UI, real forge, OS cleanup or complete parity claim.

## Resume point

Finish current narrow checks and run comparison/private-index library tests.
Tests cover mixed staged/unstaged/untracked files, unborn HEAD, base errors,
index bytes, recreated base files (identical/modified), literal path names,
staged rename preservation, argument/copy budgets, modes and no-follow access.
Then implement the owned runtime/cancellation boundary and register pending
Quality B rows in the single method table. Complete revision/cache/watch,
classified history, branch stash metadata, merge-to/from-base and generated
commit behaviour. Root whole gate and V3 review occur once the node package is
complete; app package verification remains separate.
