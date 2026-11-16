# W16 private managed-artifact identity prerequisite

This is a proposed, non-activated source-acquisition prerequisite under ADR0079.
The existing declaration inspection binds only two decoded files. It cannot
qualify a helper executable, build result or complete reviewed candidate.

[Decision Log]
- Purpose: bind review/apply to the actual complete private candidate rather
  than treating manifest/client hashes or a runtime generation as artifact truth.
- Assets/constraints: reuse canonical PluginId, ContentHash, PackagePath,
  SHA-256, existing confined workspace path policy and the node account. The
  saved-envelope owner still owns persistence; no metadata file/writer is added.
- Alternatives: hash only declarations, trust mtime, or add a complete private
  candidate-tree observation before activation. Only the latter binds executable
  and asset changes, including same-size edits with restored timestamps.
- Choice: a bounded deterministic tree observation plus exact re-observation.
  It remains private/staged until an actual acquisition/review producer exists.
- Why: filesystem observation alone cannot mint source revision, human consent,
  settings permission, executable authority or an authenticated node operation.
- Impact: additional bounded I/O and explicit refusals. No package download,
  command/build/helper, install mutation, generic settings field or RPC is added.

## Concrete shape and limits

The artifact owner defines the constants once in its module; tests use those
constants. At most32768 entries (files, directories and links),256MiB per regular
file,512MiB aggregate regular-file bytes,4096 encoded bytes per relative name or
link target, and64 directory levels. Names must be representable in the native
package path contract. Every counter uses checked/saturating arithmetic and
refuses before reading/copying excessive data; no truncation becomes identity.
These are native resource limits, not claims that arbitrary npm trees fit.

An ArtifactIdentity contains a private SHA-256 tree digest, counts and byte
facts. Each record includes its kind, relative path, executable-owner bit and
content hash or symlink target spelling. Entries are sorted deterministically;
length prefixes and distinct kind tags prevent ambiguous concatenations. The
root and every entry are observed twice around content reads, and a complete
second observation must agree before returning a stable identity. This detects
observed changes; it does not eliminate same-account path/open races.

No symlink directory is traversed. Each link must resolve inside the same
candidate root; dangling, cyclic/unresolvable or escaping links refuse. The
link itself is hashed and its actual in-tree target is independently covered
by ordinary tree entries. Special files/sockets/devices/FIFOs refuse before
opening. Hard-linked files count/read per path; aliases do not evade budgets.
Do not block on a FIFO or follow an uncontrolled filesystem mount traversal.

Private-tree admission checks the actual effective node UID against ownership
and refuses group/other-accessible candidate directories. Regular files must
not be writable by another account; executable bits are observed, never granted
by this reader. Initial implementation uses the workspace-owned nix dependency's
safe effective-UID API on Unix, with that narrow feature explicitly declared.
No new external crate, shell command, chown or permission repair is introduced.
On platforms without a concrete private-owner/ACL verifier it returns a typed
Unsupported boundary. Recognized release ABI eligibility in the manifest is
separate from this installation qualification; it does not waive ownership.
Windows activation cannot consume a Unix-mode inference or existing Store
privacy fallback. A later reviewed verifier must qualify that path explicitly.

## Errors, lifetime and authority

I/O/parser errors retain only sealed safe classifications through Display,
Debug and every exposed source. Candidate names, local paths and file contents
must not appear in diagnostics. A node-owned blocking task owns a complete
observation through return even if its waiting RPC caller disappears. Cancellation
is checked between bounded reads; the eventual actual admission lane must keep
its permit until the blocking task settles. Dropping a wait does not delete a
candidate or release a process/settings lease.

The first implementation is private under the existing plugin staging gate,
with real disposable filesystem tests. It does not create a managed directory,
copy/acquire packages, write settings or call Supervisor::spawn. A future
acquisition owner alone creates its candidate, supplies the exact private root,
records pinned Git/npm/registry provenance and produces a review proposal.
ArtifactIdentity is a captured fact, not that producer's unforgeable authority.
Apply must re-observe the exact owned candidate and revision, then use the
reviewed start/close fence and sole settings owner. Directory sources are never
deleted by candidate cleanup. Full source/build/storage/platform qualification
remains separate.

## Required evidence

Actual private temp-tree cases cover deterministic order, ordinary files and
executable distinction, same-size edits with restored time, path/byte/count/depth
refusal, internal links versus escape/dangling links, FIFO refusal without open,
changed-tree refusal and full diagnostic canaries. UID/GID/modes are inspected
under the test runner account, without changing an account or service. Generic
Windows refusal is not a Windows ACL or runtime pass. Existing declaration-only
inspection and the actual saved-owner/lifecycle integration retain their own
separate evidence obligations.

The observation budget is30s, checked between64KiB regular-file reads and tree
entries. Expiry is a typed refusal; it never turns a partial observation into
identity. A blocking filesystem syscall cannot be forcibly interrupted by that
check: its owned task and admission remain tracked until it actually settles.
Entry device identity must agree with the candidate root, so this observer does
not descend into another mounted filesystem. Acquisition must prepare an owned
package tree without VCS control metadata before this package-path observation;
it must never strip metadata from the user's directory. Pinned source revision
is recorded by that separate owner, not reconstructed from an artifact digest.


## Pure fact-model checkpoint (implementation still staged)

The private plugin module now has the deterministic supplied-record digest and
atomic count/byte/path/depth budget model. It reuses the existing PackagePath
validator through plugin-private visibility, ContentHash and SHA-256. Three
pure tests are authored for ordering/kind/content/execute distinctions, duplicate
paths, every counter refusal retaining prior facts, and full-chain invalid-path
canaries. They allocate no candidate filesystem and execute no process.

TreeDigest explicitly means supplied facts, not a complete/stable filesystem
receipt or source authority. No complete tree reader, root/ACL verifier, blocking
worker, admission lane or installation is implemented by this checkpoint. Their
proposed contract and real-OS evidence remain pending; the later file probe below
is narrower.
No new protocol/storage type or public API is added. V3 reviewed the selected
pure model at c4a3c792. Run wp-plugins-artifact-test-20261115T180604Z-c4a3c792
executed all22 staged plugin tests with no failures, including the three new
pure cases; all prerequisite steps passed. Its paired strict check refused one
test-only manual match, corrected to equivalent let-else without changing the
canary assertion. Changed strict remains owed. None of this executes the future
filesystem observation or turns supplied records into acquisition authority.


## Staged Unix regular-file probe

[Decision Log]
- Purpose: refuse special files before open and check the actual descriptor
  before reading, including a FIFO substituted after the pathname check.
- Reuse: std OpenOptions/Metadata, existing SHA/ContentHash, and workspace nix
 0.31.3 safe UID/flags. Store's attachment reader already uses nonblocking,
  no-follow and fstat, but is private to stored attachment IDs and cannot admit
  arbitrary package files. No shared Store API is widened or copied as a parser.
- Choice: a private synchronous probe under the existing literal plugin test
  gate, with Unix-only dev dependency nix user/fs. The user feature already
  exists through proc; fs has no additional dependencies in the pinned crate.
- Why: an ordinary open after lstat could block on a substituted FIFO; the
  opened descriptor must independently be regular, same device/UID/mode and
  unchanged inode/size/mtime/ctime before and after bounded streaming hashing.
- Impact: one local lock edge to the already pinned nix0.31.3; no new package,
  normal product dependency, process, permission repair, RPC or installer.

Probe.prepare captures an ordinary file's metadata, checks actual effective UID,
refuses group/other write, device mismatch and the per-file byte cap. Probe.read
uses O_NONBLOCK|O_NOFOLLOW, verifies fstat before any read, hashes in the named
64KiB chunks, checks cancellation and the30s deadline between reads, and checks
both descriptor and final pathname metadata. A failed/expired observation never
returns a partial content hash. Concrete I/O evidence is retained behind a sealed
safe kind/OS-code diagnostic whose public source terminates before raw text.

This probe DOES NOT validate a private candidate root, parent directories, path
confinement, complete entry/link coverage, aggregate budgets or two whole-tree
passes. It is a prerequisite for that future owner, not an ArtifactIdentity or
source authority. It accepts only the owner's private local path in this staged
module; no external caller can use it. There is no Windows API or ACL fallback.
No blocking task/admission permit is introduced yet, so caller-loss/shutdown
settlement remains an actual integration obligation, not supplied by this probe.

Four disposable-filesystem regressions are authored: actual descriptor flags and
same-size content edit with restored mtime; direct and substituted FIFO plus
symlink refusal; sparse oversized file/device mismatch/cancel/expiry; and every
safe diagnostic/source level retaining a private concrete canary cause. A FIFO
peer opened nonblocking keeps the substituted-FIFO fixture bounded even if that
flag regresses; the separate actual-descriptor assertion checks NONBLOCK itself.
The tests assert runner UID/GID and initial0600 creation mode without chmod/chown.
They do not manufacture or claim actual wrong-account ownership, Windows ACL,
root/parent privacy, descendant lifetime or complete-tree proof. Actual new-source
strict/tests and V3 selected implementation review remain owed.

The first file-probe suite9e460e91 reached the node but stopped before any new
case at an exhaustive declaration-error classification after the file package
added four FileError variants. Both strict and test failures are retained;
all test prerequisites succeeded. The classification now names all four fixed
safe categories and adds a concrete invalid-entry canary through every exposed
source level. This corrects the receiving-base join, without relaxing the probe,
changing shared file errors or claiming the new filesystem cases executed.

## Staged synchronous whole-tree observation

[Decision Log]
- Purpose: replace a supplied-record assumption with two actual bounded tree
  observations, including helper/assets, private directories and link coverage.
- Reuse: the existing workspace Root canonicalization, canonical PackagePath,
  supplied-record hash/budget and the checked regular-file probe above. The
  budget admits unread shapes without fabricating a placeholder content hash.
- Choice: private Unix Pending state after a full first pass; finish performs a
  complete second scan/hash and compares both content facts and private metadata
  evidence before returning the digest. Pending is never successful identity.
- Why: hashing just two declarations, trusting mtime, or checking a partial list
  cannot bind all helper/assets. A same-size edit with restored mtime still changes
  the content hash; private inode/ctime evidence detects additional observed races.
- Impact: bounded synchronous blocking I/O, no acquisition, worker, persistence,
  RPC, caller authorization, host start or executable-loading capability.

One30s deadline and cancellation token span preparation and both full passes;
file readers receive that same deadline rather than resetting their budgets.
Each scan admits the complete count/depth/per-file/aggregate logical-byte plan
before reading file content, so three individually allowed sparse files cannot
force reading more than the total budget. Special files refuse before any content
open. Directory links are never traversed. A link's canonical target must remain
in-root and be covered by a non-link entry or the observed root; the target set
uses borrowed validated names rather than a quadratic path-joining search.

The actual root and every directory must have the effective runner UID, no
other-account permission bits, and the root device. Files must have that UID,
no group/other write and the same device; every opened file repeats descriptor
checks. Links retain their spelling and must have that UID/device as well.
The original workspace Root is reused; private wrappers retain its concrete
PathError and unrepresentable OS names without exposing their text/chain. Final
path/descriptor/root stamps and the second complete pass must agree. Metadata
is private comparison evidence, not another persistent artifact/provenance schema.

This detects observed changes but does NOT eliminate same-account pathname/open
races. Equal st_dev is not proof against same-device bind mounts. There is no
Windows ACL verifier or advertised generic fallback, and no command/helper is
executed. The synchronous observer has no detached thread/lane: a future actual
blocking owner must retain admission through settlement even after caller loss;
its between-read checks cannot interrupt a kernel syscall. That lifetime join,
source pin/review/consent, saved namespace, start fence and all product integration
remain separate obligations before activation.

Five real-FS cases are authored: two roots created in different order with leaf
and directory aliases; same-size restored-mtime edit held between full passes;
escape/dangling/cyclic links and FIFO; aggregate sparse-byte and real depth refusal;
root-link/cancel/shared-expiry and actual invalid OS-name/full-chain canaries.
Only initial private fixture creation/set_len/FileTimes/remove-owned fixtures is
used: no chmod/chown, account change or process. Tests do not claim actual
wrong-account ownership, mounted-device or other-platform qualification. These
new cases, current strict and the selected security delta remain unexecuted until
the next meaningful suite; prior regular-file/pure-model evidence is separate.

The62c592af regular-file suite actually ran27 cases:26 passed and the held
same-size/restored-mtime probe's Changed-only expectation failed. Its raw log
contains no returned value or before/after stamp, so timestamp granularity is
only a hypothesis, not established cause. A test-only diagnostic records the
synthetic fixture's numeric metadata and typed result while retaining the exact
assertion. It contains no real path/content/credential. The full-tree extension
compares two content hashes independently of this metadata expectation; neither
has new execution proof yet. The paired strict62 run also refused unchecked
Instant addition, a64KiB stack buffer and an unconsumed seek result. The extension
uses checked deadline construction, a bounded heap buffer and asserts the actual
seek receipt; those changes preserve refusal and the named budgets.
The successful two-pass result is a distinct privately constructed Observed
wrapper. Supplied-record TreeDigest values cannot construct it; only Pending's
complete re-observation does. This is a filesystem observation distinction,
not installation/source provenance or executable authority.

The diagnostic e149e7a1 run supplied actual cause evidence: all recorded metadata
fields before and after the same-size rewrite were identical, including ctime,
and the prepared probe returned the fresh hash of the replacement bytes. A
metadata-only preparation has no old content hash to compare; requiring Changed
in that observed state was an invalid fixture expectation. The corrected case
requires the new hash when stamps are equal, still requires Changed when they
actually differ, retains old-versus-new hash inequality, and adds an unconditional
held size-change refusal. It never permits reuse of an old content hash. The
whole-tree held-between-passes case independently requires Changed from the two
actual complete content observations, even if timestamps happen to match.

All five tree cases in that run also failed before their intended boundary:
TempDir's pinned default is0777 filtered by umask (actual fixture0755), not0700.
The existing private-directory policy correctly rejected it. Fixtures now request
0700 at initial creation using the pinned Builder permissions API, whose Unix
implementation calls DirBuilder.mode before create; no chmod/repair or product
permission relaxation is used. UID/mode and all policy assertions remain. The
same run's three strict failures (doc backticks, temporary diagnostic printing,
open enum wildcard) are corrected without allowances. Original26/6 runtime and
strict-red logs remain separate from the next corrected-source proof.

## Owned blocking observation join

The synchronous tree observer cannot make caller-loss lifetime claims by itself.
Its next private adapter accepts the caller's actual OwnedSemaphorePermit and
existing TaskTracker; it creates no semaphore, independent queue or settings
writer. One tracked spawn_blocking closure owns that permit, local child
cancellation and the real full observation until the filesystem work returns.
Dropping a result waiter requests child cancellation but cannot release the
permit or detach ownership before actual settlement. The parent/node token is
not cancelled by waiter disposal. The result is published only after synchronous
I/O settles and its permit is released; errors stay typed and input-free.

TaskTracker.close is NOT admission authority (it still allows tracking work).
The actual future plugin manager must acquire its permit under its serialized
node/lifecycle admission fence before calling this private adapter, and retain
its qualified candidate/root lease as well. This adapter neither supplies that
fence nor proves candidate acquisition ownership. No startup, settings apply,
helper launch or new cross-crate API is added. Runtime shutdown must drain the
same existing tracker before candidate/store teardown; a blocked kernel call
still cannot be forcibly cancelled by this observer's between-read checks.

Actual tests will hold one blocking worker before real observation, drop only
its waiter, and prove the real permit and tracker remain occupied until the
held worker is released and settles. A separate success case runs both actual
filesystem passes through that same adapter. No fake observation result or
second filesystem path is kept beside the production function. These are
planned lifetime checks, not executed evidence or a real NodeHost admission test.

The private adapter is now authored as Observation with a tracked JoinHandle,
not a second result queue. The existing tracker owns one real blocking closure;
its permit guard drops before actual join completion. Cancelling/dropping a polled
wait requests only its child token and does not abort/detach the tracked work's
ownership. A concrete JoinError remains behind a safe cancellation/panic wrapper;
its public chain never reopens panic text. Missing runtime and already-cancelled
parent refuse before tracking or I/O. The absolute deadline is captured before
spawn, then passed to the same synchronous reader: waiting for a blocking-pool
thread cannot restart the30s budget.

Six tests are authored for actual full-FS success, a held real worker after
polled-waiter loss retaining its permit/tracker until settlement, captured-expired
deadline refusal before I/O, absent runtime, pre-cancelled parent, and an actual
synthetic panic JoinError with sealed diagnostics. Named test wait budgets bound
their own async waits. None proves a serialized NodeHost start fence, private
candidate acquisition ownership or real service shutdown. Source/narrow/containing
package proof remains owed; the adapter stays under literal plugin staging.

The corrected711d0ab0 synchronous observer has now executed all32 staged plugin
cases successfully with every prerequisite step passing, paired with all-target
node strict success. This includes both real full passes, private-root fixtures,
link coverage/refusals, sparse/depth admission and fresh-content/timestamp cases.
Those exact-source results do not execute or accept the later owned adapter.

Row296 returned the test clocks, not a bounded-operation product flaw. All five
async cases now enter canonical vessel_testkit::within after fake preparation.
One named90s case clock encloses a60s body safety clock and the same tracker
cleanup, with distinct named10s worker-entry/result/drain phases; the product
observation remains30s. The body is caught only to close/drain that original
tracker before resuming its exact panic or failing its timeout. Dropping the
body still disconnects the held worker's original std sender; no second worker,
reaper or fabricated settlement is added. Every old permit/tracker/cancellation/
canary assertion remains, and missing-runtime stays a synchronous test. Parent
module descriptions now name staged filesystem and blocking work accurately.
Actual changed-test strict and execution remain owed; prior38-case evidence is
preserved as predecessor execution, not borrowed for the clock correction.
