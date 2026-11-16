# W16 owned candidate boundary (proposal; no producer yet)

This contract separates a filesystem handle from source provenance and human
approval. It proposes the next private prerequisite after the staged artifact
observer. No implementation, public API, acquisition, installation, settings
field, process launch or cleanup authority is introduced by this document.

[Decision Log]
- Purpose: retain the exact newly acquired candidate through observation,
  review, apply or refusal without treating an arbitrary path as owned storage.
- Existing assets: canonical PluginId, PackagePath and ContentHash; the staged
  two-pass artifact observer and its tracked blocking operation; DataDir's
  configured location; existing nix0.31.3 safe descriptor APIs and UUID ids.
- Constraint: DataDir is a path value, not proof that the path, ancestors or a
  candidate are private. A tree digest is not acquisition provenance. The sole
  settings writer and the actual host lifecycle retain their separate owners.
- Alternatives: reopen absolute paths, infer ownership from a digest/device id,
  or retain a verified directory descriptor plus exclusive candidate creation.
- Choice: a private retained descriptor boundary with explicit refusal, followed
  by an independently qualified acquisition owner. Keep both staged until their
  actual consumers and applicable platform evidence exist.
- Why: path spelling, inode observation, process generation and consent each
  establish different facts. Combining their names must not grant new rights.
- Impact: additional descriptor ownership and fail-closed platform requirements;
  no second parser, settings writer, filesystem repair or unsafe wrapper.

## Distinct values and ownership

The eventual node assembly selects the configured data location. A proposed
`CandidateStore` may only open the fixed plugin staging namespace under that
location after verifying its actual ownership and ancestor policy. Taking an
arbitrary helper-provided path or a previously observed digest is insufficient.
The concrete ancestry/bootstrap check is a separate required code checkpoint;
neither DataDir::new nor the existing Store privacy fallback supplies it.

The proposed Linux bootstrap starts from a retained filesystem-root descriptor,
walks the configured absolute data path without following symlink components,
and verifies each actual ancestor descriptor: owner is root or the effective
node UID, with no group/other write permission. The final data directory and
plugin staging directories must belong to the effective UID with no group/other
access. Existing insecure entries refuse; there is no chmod/chown repair. Shared
sticky ancestors and symlinked data locations also refuse under this stricter
native policy. Mount boundaries on this trusted configuration bootstrap are
distinct from descendant package traversal; NO_XDEV begins at the retained data
root. Tests must distinguish those two paths rather than claiming that a single
open of the final directory verified its ancestors.

A private non-Clone `Candidate` owns the exclusively created random candidate
entry, its retained directory descriptor and the parent/name identity needed
for bounded cleanup. It cannot be deserialized or created from a path, UUID,
TreeDigest or caller-supplied record list. It grants access only to that owned
entry; it carries no registry credential, source revision or helper permission.

The acquisition owner retains the candidate while producing actual Git/npm
source facts and materializing package bytes. That owner must pin the concrete
revision/artifact, enforce transport/resource/credential policy and retain its
real command or transfer work through settlement. This descriptor prerequisite
does not implement those operations. Registry intent and provenance stay
distinct, including a registry-selected exact artifact and user-selected source.
The existing source parser remains spelling-only until that owner consumes it.

Only after acquisition/build has settled may the owner request complete artifact
observation. A proposed `ReviewedCandidate` binds the same retained candidate,
actual pinned source facts and observed artifact to the precise review request.
It cannot be produced by the observer alone. Apply rechecks that request and
reobserves the retained candidate before the sole saved-owner and start fence
can consume it. Local directory sources never enter managed deletion ownership.

## Descriptor and platform policy

On Linux, prefer the existing safe nix openat2 API for descendant operations,
with BENEATH, NO_XDEV, NO_MAGICLINKS and NO_SYMLINKS resolution. NO_XDEV is the
kernel mount-crossing policy, including bind mounts; comparing st_dev alone is
not an equivalent test. Initial file/directory modes are private and actual
descriptor metadata is verified after creation/open. Special entries refuse
before content I/O; O_NONBLOCK and actual descriptor verification remain required
for regular files. Symlink records retain their separate bounded observation
policy; this proposal does not silently replace it with symlink traversal.

The root bootstrap and each later use must retain their explicit guarantees.
An open root descriptor does not retroactively verify an insecure ancestor
lookup. Even verified descriptors do not prevent same-account content mutation,
which still requires complete reobservation; no general same-account race-free
installation or immutable filesystem snapshot is claimed.

Unsupported kernel resolution flags must return typed Unsupported without
retrying with weaker flags or pathname fallbacks. This Linux design does not
qualify Darwin, Windows ACLs or either release ABI by implication. Additional
nix features, if necessary, need their exact manifest/lock/source review before
implementation; the current Unix dev edge is not a normal product dependency.

## Lifetime, cleanup and failure

Use the caller's original admission permit and existing tracked operation owner;
do not add a competing acquisition queue or worker tracker. The same operation
must retain the candidate lease through actual acquisition/observation/cleanup
settlement, even when its reply caller disappears. Cancellation is cooperative
and cannot release a permit while a syscall or child still owns work. Host
shutdown must close actual admission before draining that work.
The current observation adapter retains a permit only through observation; it
does not yet implement this larger acquisition/apply lifetime. Its integration
must preserve that ownership rather than borrowing its successful result as a
new permit. Awaiting human review retains candidate ownership as explicit bounded
state, not a running syscall or an indefinitely held settings-writer guard.

Drop must not recursively delete a saved path or start untracked blocking work.
Explicit cleanup is confined to the exact exclusively created staging entry,
uses retained parent/entry identity and refuses replacement or unexpected mounts.
It never follows links into another tree, deletes a user directory, repairs
permissions, or treats failed/unfinished cleanup as success. Failed settlement
keeps an explicit unavailable/cleanup-pending outcome; eventual crash recovery
requires its own persisted ownership contract, not a scan-and-delete heuristic.
Identity comparison refuses an observed replacement; it is not an atomic
unlink-if-inode operation. A same-account rename between comparison and removal
remains outside this guarantee. If the required owned-entry evidence is absent,
cleanup refuses instead of searching by a stale saved pathname.

Concrete I/O/kernel causes remain privately retained. Public Display, Debug,
serde-facing diagnostics and every exposed source level reveal safe closed
classes only, without paths, source URLs, credentials or file content.

## Required proof before any producer claim

Use disposable private fixtures under the numeric runner account. Cover actual
descriptor ownership/modes and exclusive creation, unexpected file/link roots,
root/entry replacement, traversal and unsupported-kernel refusal, held work after
caller loss retaining the same permit/lease, and cleanup refusing a substituted
entry while leaving an external sentinel intact. Clock async cases with canonical
within and shorter named phases; failure must settle the same owned work.
Actual bind-mount qualification needs an authorized isolated mount fixture; flag
inspection or st_dev-only tests are not executed mount proof. No service, account,
mount, chmod/chown or real-source operation is authorized by this plan alone.

Paseo references: managed-source.ts prepareInstall/place/createStagingRoot and
protocol/plugin-source-reference.ts. Their version-root UUID, source-pinned
candidate and directory-vs-managed distinction are reused as ownership intent;
the native descriptor policy is an explicit stricter platform requirement.
