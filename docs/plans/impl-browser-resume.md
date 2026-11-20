# Browser packet resume

2026-11-20. The quota stop applies during P1 (TRACKER 512). The current slice
is the Linux Chromium component, latest visible-frame slot and workspace-bound
target inventory. P2–P5 have no implementation commits from this track.
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
- The live artifact export for `0e86fb5c8` was queued at this checkpoint as
  `impl-browser-node-20261120T035735Z-0e86fb5c`. Query its state and recover
  the existing result. Do not submit a duplicate. At this checkpoint the
  added frame-slot and target-ownership checks have no live CFT result.

## Review and next work

The coordinator approved the plan and ADR and received the scoped pass for
`41682f25c`. No node, app, integrator or visual review is open. When P1 is
ready, the node reviewer precedes the integrator. P2 onward needs the
coordinator's frames verdict before the native reviewer and lead.

1. Settle the exact target/visible-slot result and retain any failure receipt.
   A passing selected fixture still leaves the ordinary browser suite and
   exact whole root gate to run before package review.
2. Finish P1's node actor and real event routing. Consume the prepared child
   and workspace targets through the single bounded dispatcher, and keep
   discrete state/terminal notices separate from the latest frame. Test actual
   shutdown and subsequent profile reuse under the service identity. Run the
   full General network qualification against controlled local fixtures,
   including redirects, subresources, service workers, DNS, ICE and
   QUIC/WebTransport. The isolated diagnostic's two loopback HTTP refusals
   are scoped evidence. Keep methods pending and the engine hidden until
   qualification and actual integration.
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
