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

The existing workspace-pinned tempfile dependency is now a normal dependency
for the complete comparison read API; ADR0087 and ARCHITECTURE record its
license, alternative and tradeoff. Each entry point awaits private-index cleanup
on success and failure. Bound cancellation reaches all helper commands. Node
TaskTracker ownership and its bounded execution queue are the next caller;
there is still no new admitted RPC, cache or transport.

## Verification boundaries

- Initial contract and committed resolver at 3611a6b4: protocol/workspace narrow
  clippy green; library tests 382 passed, 0 failed, 1 ignored.
- All/private-index at 1bf60f28 passed all-target workspace clippy, then its
  library tests returned 110 passed, 1 failed, 1 ignored. The failing actual
  filesystem assertion found a 0755 scratch directory, because tempfile's
  directory default is 0777 masked by the process umask. The corrected builder
  requests 0700 at creation, before copying any index bytes; no process-wide
  umask or post-creation chmod is used. Its changed-source check is pending.
- Logs: `wp-git-node-narrow-20261115T072022Z-1bf60f28` (clippy),
  `wp-git-node-narrow-20261115T073212Z-1bf60f28` (tests). Both compiled the
  changed workspace crate. These are staged library checks, not node admission.
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


Latest executed staged result at614d8026: workspace library111 passed,0 failed,
1 ignored, including actual Unix0700/0600 assertions. Run
`wp-git-node-narrow-20261115T074017Z-614d8026`. This is the fixed private-mode
source, not proof of the later complete-read entry points or their node caller.
The complete-read API adds real-git tests for paired summary/body cleanup and
cancellation after index preparation; those changed-source tests are pending.


## Owned comparison caller and pending registration

The next coherent source connects complete comparison reads to the node's
existing root authority, TaskTracker and shutdown token. Its separate admission
bound is64 including queued reads, with4 executing snapshots. A dropped reply
receiver never drops a running cleanup future. Shutdown refuses queued work,
cancels every executing git helper and waits for their cleanup. Barrier tests
cover receiver loss, execution/admission limits, queued cancellation and cleanup
ordering; real repositories cover node-level comparison semantics, invalid roots
and unchanged index bytes. These new tests have not been executed yet.

`git.compare` and `git.compare_file` have Read/unary rows in the one Quality B
table, pending `workspace::GIT_PANEL = None`. They route real workspace methods;
no dummy version, admission claim or stub success is added. NoBase, TooLarge
and a genuinely empty Ready remain distinct typed values. Partial summary
coverage is retained. The client in-memory transport test asserts no frame is
sent while pending. Agent access is explicitly refused like the existing git
calls. Request/reply, older projection, fake handler and alias metadata changes
are the minimal exhaustive registration consequences.

The previous complete-read narrow check (0a391e7f) returned only an unquoted
DiffScope name in documentation. The current source quotes it and needs its own
changed-source check. Revision/cache/observation/history and write policy are
still incomplete, and the package is neither built nor passed. The web wire
must be regenerated from the protocol generator before package closure.

The first pending-registration check at6e9db435 stopped in protocol compilation:
three exhaustive forwarding/field-admission/older-reply matches needed explicit
comparison arms. The remaining method sample and bridge classification were
found in the same audit and updated; no wildcard or broadened fallback is used.
A file-body request also rejects the relative root before any job is admitted.

An isolated real-git permission probe found another private-index issue before
claiming runtime readiness: git replaces an index via a lock file and can widen
its mode to0664 despite a0600 copy (even with sharedRepository=false). The
private-only update-index command now uses the documented numeric
`-c core.sharedRepository=0600`; a separate probe kept0600. This does not alter
repository config or the real index. The actual protected-reader regression
now uses a group-sharing repository and asserts0700/0600 after summary/body
reads and unchanged real-index mode/bytes. It remains unexecuted in this tree.

At a9301381 the full selected all-target narrow check compiled the production
protocol, workspace, RPC, client and node code. It returned nine test-only
unused-result errors (tracker close booleans and fixture git stdout). Tracker
closure is now asserted and quiet setup outputs are explicitly acknowledged.
No lint allowance or production behaviour was changed by this correction.
Those fixes and the group-sharing permission regression still need their
changed-source narrow check and execution. Before the eventual whole root
gate, merge exactly the reviewed base security-test race fix c5da5305; do not
edit that test locally or relabel its base failure as feature evidence.
