# W20 E4 resume — protocol 24 consumer, quota stop

## Start here

The active worktree is `wp-onboarding`, branch `backend/m12-wp-onboarding`.
Read this note, the current WORKER/PLAN ownership and remote run states before
changing anything. Do not resubmit a queued, running or completed run. All Rust
compilation, Cargo metadata, checks, tests and capture builds stay on the build
VM through `vessel-remote-gate`; local static rustfmt/size/tokens are permitted.

The warning required keeping 30 percent of weekly quota with no extra reset.
The current slice is committed and coherent, but the APP/whole/UI gate cannot
finish in the remaining window because the APP check is still ordinary queued.
No new slice, control, branch merge or long build was started after the warning.

## Committed source and exact scopes

- Approved receiving APP base: `67637eb6` (base 14.3). Never merge a node head
  into this APP tree; wait for an announced APP line at the next natural point.
- Staged SDK leaf: `d6cd205801bb8dfc24630d95a34d8deb077345ec`, V1 row287 FINAL
  bounded staged source/Link narrow PASS, strict zero and SDK framed 4/0/0.
- Straight receiving SDK leaf on approved base 14.1:
  `41215ebfc97819815a036ffa3ecc79054034fc89`, V1 row287 FINAL bounded receiving
  staged source/Link narrow PASS. Actual counts are 4/0/0, **120 filtered**;
  an initial 126 filtered was corrected from raw logs. No rerun was needed.
  `c27cf137b6fa1ebf82c7ba0367f7e7db0e251843` is its documentation-only receipt.
  The lead merged this staged source into base 14.3.
- `7cbe59d0f8cb17c8b1eec0298b08d6efb9f2b261`: atomic real E4 collector, sealed
  answers, original-context/weak-receipt checks at queue/install/frame/copy,
  independent basic and SDK reads, partial process data, APP Scenario and Link
  timeout/queue/drain regressions. Only the SDK method's staging attribute and
  obsolete staging comment were removed; its body and the existing guard,
  call/follow/protocol/delivery/exports remain unchanged. The SDK test module
  remains `cfg(test)`.
- `866250937`: required base 14.3 merge. Diagnostics and CodeSource event arms
  are unioned; History retirement/Fleet replacement ordering is preserved.
- Current tested executable source:
  `8726efe19f299d3cc1512a940e37e9564ac50a5f`. It fixes three test Call wildcard
  lint errors using an explicit NodeInfo/Diagnostics assertion; unexpected RPCs
  still fail. The same APP fixture pattern was fixed before its first check.

Existing Settings Diagnostics is extended, not duplicated. Provider diagnostics
belongs to A9; onboarding belongs to W15. G2 has only the delegated jiff About
attribution line; W20 made no About change. No new RPC, node implementation,
provider refresh, raw log tail, arbitrary process inspection or epoch allocator
was introduced. Source/module/data-flow rationale is in
`wp-diagnostics-producer.md`; the isolated SDK rationale is in
`wp-diagnostics-sdk.md`.

## Completed checks, not broader approval

On exact `8726efe1`:

- `wp-onboarding-sdk-20261116T032035Z-8726efe1`: fonts and native-cargo zero;
  actual View/Link checking, Link all-target Clippy with warnings denied passed.
- `wp-onboarding-sdk-tests-20261116T032157Z-8726efe1`: fonts and native-cargo
  zero, actual Link compilation, **16/0/0, 120 filtered**. These include SDK4,
  real held-eight-read deadline/permit reuse, actor/egress drop drain before
  peer closure/SDK deadline, full queue closed/retired refusal, supported partial
  process data, old-peer zero diagnostic frames, identity/refusal redaction and
  same-node reconnect notice behavior. Evidence manifest: scratch artifact
  `consumer-link-proof.json`; raw folders `consumer-link-clippy-2` and
  `consumer-link-tests`.

Earlier view slice `4e65312f` passed 28 diagnostic tests and View all-target
Clippy on base 13. That is historical exact-source evidence, not a current APP
or whole-union pass. L accepted the final diagnostic-flow explanatory framing
and Stopping wording/context on `07ea44bd`/`87cbebe3`; catalogs were not changed
later. No new wording review is needed unless those strings change.

The old V4 row173 bounded producer-unavailable package (whole1302/0/2,
corrected1304/0/2 and 36 captures from source50) is historical. Do not attach its
approval, running screenshots or privacy/copy claim to this actual SDK consumer.

## Pending VM run — first action on resume

`wp-onboarding-app-20261116T032035Z-8726efe1`, label `wp-onboarding-app`,
ordinary lane, exact source `8726efe1`, locked
`clippy -p vessel-app --all-targets -- -D warnings`.

It was QUEUED at 03:37 UTC (last retained queue position33 at03:35).
Evidence output `consumer-app-clippy-base143`; follow/attach logs use that name.
Read its state and retrieve finished logs; **do not resubmit**. A local attach
observer is stopped for the quota pause; the remote job is retained.

The previous APP run `wp-onboarding-app-20261116T031210Z-7cbe59d0` was verified
queued and cancelled only after the mandatory new base made it obsolete. Its
terminal143 is supersession, not a compiler result. The prior Link run on7cbe
failed native-cargo101 at three test wildcard arms; preserve those original
logs in `consumer-link-clippy`. They were not runtime passes.

## Still unverified and next steps

1. Finish the pending APP all-target strict check. Fix actual failures with
   narrow immutable-source runs only. No APP Scenario, clipboard, refresh or
   copied-notice runtime execution has happened for the new consumer yet.
2. Run current APP diagnostic/receipt/language/refresh tests using the existing
   G4 Scenario and Shell test facade. Do not fabricate guards or add another
   actor, codec, authority constructor or allocator. Scenario close ACK drains
   Devices8 only; E4's separate actual dispatcher drain regression is the Link
   proof above, not a borrowed generic close guarantee.
3. Run current consumer wasm checks and the final whole native gate after
   narrow checks pass. Freeze that base/source through capture, review and
   handoff. No current whole gate/build or post-promotion V4 review exists.
4. Obtain actual same-source app, node and fake capture binaries through the
   supported export. The full export contains app/vesseld/six fakes but **no
   vessel CLI**, while fleet.sh needs vessel. Plan the supported same-commit
   `--root-binaries vessel` export too, or independently verify a same-root CLI
   artifact. Do not present protocol23 shared binaries as protocol24 producer
   proof. No new capture build/export was submitted before the quota stop.
5. Use isolated paired fake nodes and an authenticated private Xvfb display;
   no real agents, live preview/ports or pairing links typed into UI. Capture
   current wide/compact/mid EN/KO dark/light page and report layouts, inspect
   every capture and verify the current clipboard/report boundaries. No current
   successful full-report UI, new layout, Linux clipboard/privacy or OS claim.
6. Submit the full actual-consumer package to V4 after its own gate/captures.
   V1 row287 covers the bounded staged SDK leaf and its receiving narrow only;
   it does not approve the APP collector/weak-receipt/privacy/UI/whole package.
   Then hand the reviewed bounded range to the lead, preserving later shared
   seams. Leave node/provider/onboarding tracker rows to their owners.

Active RTT remains an explicit justified omission. Basic node-info timing is
not a transport RTT. Arbitrary processes, environment, paths and raw logs stay
outside the allowlist; recent failures are only bounded code/time metadata.
Missing/nested-unsupported memory remains partial, never healthy zero.

## Shared ownership and cleanup

The lead still owns selected.rs slot order; W18 check_call is next. W20 reported
that selected-source editing is complete, with no additional edits planned.
The only pending production delta in selected.rs from base14.3 is method
staging removal tied to the real E4 consumer. Do not promote that unused method on the line alone.
Future workers preserve the original context, weak guard, History handler,
CodeSource registration, G4 canonical make_state and the single connection-ready
fanout. The new node head announcement is not an APP merge authorization.

No owned fake fleet, APP or Xvfb is running at the quota checkpoint. The
verified local APP attach observer was terminated; its child wait process also
exited. The remote APP run remained QUEUED afterward and is retained for resume.
Other workers' displays, SSH master and builds were untouched. No production/service/permission change or public push
was made. All task-owned source and private backups remain under UID1000.
