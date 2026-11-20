# Browser packet resume

2026-11-20. P1 (TRACKER 512) is active. The current slice covers the Linux
Chromium component, workspace-bound actor, profile relaunch and launch policy.
P2–P5 have no implementation commits from this track.
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
  selected binary export `impl-browser-node-20261120T043922Z-1b27514d` is
  pending in `/tmp/work/m12-claude-wave/browser-b-export-policy`. It superseded
  the cancelled queued export `impl-browser-node-20261120T042816Z-082778db`.
- `71bbd1d20` adds real-browser redirect, subresource and service-worker
  fixtures over the existing IPv4/IPv6 local proxy refusal harness. Remote
  format, strict Clippy and runtime checks for that commit are pending.

## Review and next work

The coordinator approved the plan and ADR and received the scoped pass for
`41682f25c`. No node, app, integrator or visual review is open. When P1 is
ready, the node reviewer precedes the integrator. P2 onward needs the
coordinator's frames verdict before the native reviewer and lead.

1. Run the exact selected export at the approved CfT path as UID/GID 1000.
   Confirm the actor's real watched frame, stale input and close, actual
   download refusal, the held profile and cookie persistence on relaunch.
   Then format, lint and run the extended network fixture. Keep one remote
   run queued or running at a time.
2. Finish the General network qualification, including real DNS, socket
   and WebTransport coverage. The browser method rows and `BROWSER_PROTOCOL`
   are unassigned. Engine availability waits for qualification and
   integration. Keep the exact whole root gate for package review after
   P1 reaches the node entry point.
3. P2 binds the native pane, verified frame transport and profile clear to
   the pane registry. P3 consumes the existing composer and image paths. P4
   delivers the complete tool path and default-off host setting. After P4,
   the coordinator assigns the D2 producer for P5. No other track is known to
   be waiting for this branch now. P2 needs this runtime, P3 needs the shared
   image/composer seam, P4 needs the reviewed anchored upload boundary and
   P5 needs the service lease.

Use the approved remote runner and one detached gate at a time. Recover work
from exact receipts. The live diagnostic uses the exported artifact, pinned
CfT and local fixtures under the non-root account. The profile is quarantined
after the diagnostic. No preview units, desktop opener, external test sites,
production pairing or local builds belong to this work.

The previous host cleanup scan used the four owned diagnostic markers and
found no remaining fixture browser, wrapper or exported-test process. This
track has started no fake fleet or virtual display.
