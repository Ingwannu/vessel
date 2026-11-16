# E1 client-book resume — 2026-11-16 03:40 UTC

Project label remains vessel. Worktree /home/dev/vessel-m12-e1-client-book,
branch backend/m12-e1-client-book. Last executable/source checkpoint is exact
ba8d58d61117cc02e605b8ad7d5dd9b3698eb7ee on node ed4cd137. This docs-only WIP
checkpoint changes no executable statement. All current preparation is committed.

## Evidence and exact remaining work

- b9fe8f99ad0f9ebb9f44c170d8b82f48e8f5a231: strict0/routes8/private11 actual0;
  V3 independently closed that staged narrow conditiondc5e8f02. Retain old failures.
- a40547ebd +ba8d: six bounded-output tests prepared. V3 SOURCE REVIEWED/STAGED,
  actual ba8d strict0; new output tests and receiving runtime are UNVERIFIED.
- T's exact helper audit is now complete (path/SHA in client-book-preparation).
  Once no other own Cargo is queued/running, use ONLY:
  vessel-remote-gate run --tree THIS_TREE --commit ba8d58d61117cc02e605b8ad7d5dd9b3698eb7ee
  --label settings2-narrow --out NEW_OUT --short --no-wait
  --cargo test --locked -p vessel-client --lib private_files
  Expected17, not executed. No OR filter, fullclient, new harness args or duplicate.
- No Root tests are queued. Sole E2 run is settings2-20261116T031322Z-927680a9,
  export recovery in /tmp/work/m12-settings4/rg-frame-export-927; query status
  before any new job. Leave queued/running/done runs intact, retrieve results.
- After exact runtime, record result and same-stage V3 review; then merge the
  then-announced Node head (bc9cacc6 was announced while stopped), never rebase.
  No new merge/run is started under the quota stop.

## Product scope is still incomplete

Existing module is literal cfg(test,unix), not a production private file API.
Live format2 decoding/writing, transactional lock/re-read/commit, worker/byte
admission, actual credential persistence/partial knowledge, opaque secure-issued
receipt and typed upsert are UNIMPLEMENTED. Windows behavior and current receiving
native union are unverified. G4 and later E1 consumers wait for those real APIs;
no endpoint/first-alias trust fallback or second store/connector is allowed.
Root-only producers ultimately go through the node integrator, not an App merge.

V3 reviewer:2ea87675-0ded-441a-b64d-61c49ee109d2. No new final runtime review
requested for ba8d because runtime is owed. T:b2f9553a-6834-433b-b91e-2f461798f4c6.
Detailed local receipts/prepared source analysis are under /tmp/work/m12-settings4.
No own fake fleet, App or Xvfb was started; isolated helper test children were
actually reaped. No production config/credential/service/account permission edits.
