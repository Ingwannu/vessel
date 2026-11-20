# Browser packet resume

2026-11-20. The coordinator moved the track from the verified P1 component
into P2 (TRACKER 509). The two remaining P1 checks move to the end of the
track. The app worktree advances independently from the qualified component.
Product browser calls, the native pane and agent tool discovery are unavailable.

## Committed source and evidence

- Node base `55aa80f06` (protocol 36, minimum 2, schema 23). App base
  `cd3112d81` (14.81, protocol 36). The app worktree has no browser edits.
- `9118ba69a` records ADR 0119 and carries the accepted runtime ADR 0078.
  Linux General has an independent qualification path. Preview depends on
  the verified service lease. Agent tools wait for the complete 22-tool path.
- `2dd25980a` carries the staged Chromium components and required browser
  value types. `9c6b81ae1` adds viewport/document input fencing. `05c9e37fb`
  adds held IPv4/IPv6 canaries behind the actual General policy proxy.
- `e2eed7551` and `41682f25c` place the child, private debugger and profile
  under one retained candidate owner. Exact `41682f25c` passed remote format,
  strict browser Clippy and export `impl-browser-node-20261120T034126Z-41682f25`.
  Its sandbox-preserving, pinned CfT diagnostic passed 1/0 in 6.75 s as
  UID/GID 1000. It checked trusted pointer/text, rejection of old frame input,
  viewport/compositor 390×844, full page 390×1200, local IPv4/IPv6 proxy refusals,
  private-pipe closure and a still-held profile lock. The post-run audit found
  247 owned private entries and zero exact-profile processes. No signals were
  sent. The per-renderer seccomp sampler recorded no sample.
- `24102e2e0` adds one rate-limited latest-frame slot. `9e0be06b8` formats
  it and records the successful component. `0e86fb5c8` adds workspace-bound
  target IDs, the existing tab cap and removal after actual close acknowledgement.
  Exact `0e86fb5c8` passed format (`impl-browser-node-20261120T035620Z-0e86fb5c`)
  and strict browser Clippy (`impl-browser-node-20261120T035711Z-0e86fb5c`).
- Exact `0e86fb5c8` export `impl-browser-node-20261120T035735Z-0e86fb5c` passed.
  Its selected live CFT fixture passed 1/0 in 6.36 s as UID/GID 1000. It
  also verified frame delivery, frame invalidation, foreign-workspace refusal
  and target removal after close. The post-run audit again found 247 owned
  private entries, no exact-profile processes and no cleanup signals.
- The ordinary focused browser suite `impl-browser-node-20261120T040057Z-0e86fb5c`
  passed on the same source with 80 passed, 0 failed and 2 ignored. The approved
  candidate diagnostic was run separately as recorded above.
- `32dc5152c` through `39f580c66` add the staged actor, notice and frame split,
  compositor scheduling and strict lint fixes. `10f5fa304` through `082778dba`
  exercise a real profile relaunch under a retained close receipt and fresh
  process observation. `b2cec50a2` records the coordinator's dated ADR 0078
  launch policy, pins lookup to the bundle, disables the session bus, admits
  HTTP(S) guest output and requires Chrome to acknowledge download denial.
- `8efd18a85` passed remote format
  (`impl-browser-node-20261120T043800Z-8efd18a8`). `1b27514d4` passed strict
  browser Clippy (`impl-browser-node-20261120T043906Z-1b27514d`). The exact
  selected binary export `impl-browser-node-20261120T043922Z-1b27514d` passed
  and superseded the cancelled export `impl-browser-node-20261120T042816Z-082778db`.
  The selected live CfT fixture passed 1/0 in 13.03 s under UID/GID 1000.
  Real actor watch, stale and foreign-workspace refusals, close, download
  refusal, profile lock and cookie persistence on relaunch all passed. The
  audit found 306 private owned entries, no named attachment or partial file,
  and zero remaining exact-profile processes with no cleanup signals.
  Evidence is in `/tmp/work/m12-claude-wave/browser-b-live-policy`.
- `71bbd1d20` adds real-browser redirect, subresource and service-worker
  fixtures over the existing IPv4/IPv6 local proxy refusal harness.
  `b19eab2e6` also fences captured frames across same-loader URL fragment
  changes. `4a0d6ad98` adds real ICE gathering against monitored TCP and UDP
  peers that independently pass local controls. `74e782dad` passed remote
  format (`impl-browser-node-20261120T045556Z-74e782da`) and strict browser
  Clippy (`impl-browser-node-20261120T045614Z-74e782da`). Export
  `impl-browser-node-20261120T045637Z-74e782da` was cancelled before acquiring
  a slot. The combined fixture uses exactly held synthetic origins.
- `66c5f9018` adds IPv4/IPv6 WebSocket refusal. `88cc01d0b` revokes watched
  targets when the real policy proxy stops. `59a744648` adds the independent
  H3/WebTransport test. Its standalone aioquic positive control passed under
  UID/GID 1000 with 5 datagrams, one TLS handshake and one CONNECT.
  `a3ceedc93` adds a system-resolved loopback name with both families held.
- `569f5e219` passed exact remote format
  (`impl-browser-node-20261120T051755Z-569f5e21`) and strict browser Clippy
  (`impl-browser-node-20261120T051811Z-569f5e21`). Combined candidate export
  `impl-browser-node-20261120T051831Z-569f5e21` passed. The focused browser
  suite `impl-browser-node-20261120T052736Z-569f5e21` passed 84/0/3. The two
  selected live tests passed separately as UID/GID 1000. Actor and network
  evidence is at `/tmp/work/m12-claude-wave/browser-b-live-full-net` (12.90 s).
  H3 evidence is at `/tmp/work/m12-claude-wave/browser-b-live-h3` (6.17 s).
  Its positive control reached 5 datagrams, one TLS handshake and one
  WebTransport CONNECT. The Chromium refusal received zero in all three
  categories. Profile audits found 310 and 240 private owned entries
  respectively, no remaining exact-profile process and no forced signal.
  The coordinator received the combined scoped pass.
- `eb5b786e2` and `eff2a9021` stage the local, isolated DNS policy fixture.
  The standalone system resolver control observed both addresses as UID/GID
  1000. Format passed on `eff2a9021`
  (`impl-browser-node-20261120T053456Z-eff2a902`). Strict Clippy failed in
  `impl-browser-node-20261120T053526Z-eff2a902` on three unnecessary
  `super::` qualifications in `candidate_tests.rs`. The new browser test has
  no export or live result. These runs are terminal.

## Review and next work

The coordinator approved the plan and ADR and received the scoped pass for
`41682f25c`. No node, app, integrator or visual review is open. When P1 is
ready, the node reviewer precedes the integrator. P2 onward needs the
coordinator's frames verdict before the native reviewer and lead.

1. P2 binds the native pane, verified frame transport and profile clear to
   the pane registry. P3 consumes the existing composer and image paths. P4
   delivers the complete tool path and default-off host setting. After P4,
   the coordinator assigns the D2 producer for P5. No other track is known to
   be waiting for this branch now. P2 needs this runtime, P3 needs the shared
   image/composer seam, P4 needs the reviewed anchored upload boundary and
   P5 needs the service lease.
2. At the end of the track, verify the app's own sandbox policy when a
   controlled name resolves to a new address and verify the authenticated
   node entry. Use local fixtures only. Keep one detached remote run at a
   time. The browser method rows and `BROWSER_PROTOCOL` are unassigned.
   Run the exact whole root gate for package review after integration.

Use the approved remote runner and one detached gate at a time. Recover work
from exact receipts. The live diagnostic uses the exported artifact, pinned
CfT and local fixtures under the non-root account. The profile is quarantined
after the diagnostic. No preview units, desktop opener, external test sites,
production pairing or local builds belong to this work.

The previous host cleanup scan used the four owned diagnostic markers and
found no remaining fixture browser, wrapper or exported-test process. This
track has started no fake fleet or virtual display.
