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
