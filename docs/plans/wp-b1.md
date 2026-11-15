# WP-B1: project scripts, services and discovered ports

Status: implementation; based on native head 1d1f11df.

## Reference and existing assets

Paseo 0.15.4, read-only reference checkout:
- `packages/app/src/screens/workspace/scripts/workspace-scripts-button.tsx:307-447`:
  named rows, a terminal link, exit badge, Run while stopped and Restart/Stop while running;
  service URLs under each row. Pending lifecycle actions are disabled.
- `.../use-workspace-script-actions.ts:66-154`: supervised start/kill; restart waits for
  stopped before starting; errors are surfaced and a failed stop cancels restart.
- `.../script-status.ts:25-58`: only running services carry health; unknown health is running.
- `packages/app/src/utils/workspace-script-links.ts:70-111`: only running services get links;
  public, proxy then direct; direct uses the connected host, with IPv6 brackets.

Vessel already has project detail, exact-digest approval and comparison in
`app/src/areas/workspace/{detail,approval}.rs`, wire calls in `link/src/workspace`,
terminal status/service state and `port.list`. These are reused. No external library
is needed: typed wire calls, GPUI's browser opener and the existing kit suffice.

[Decision Log]
- Purpose and intent: complete per-project Run/Restart/Stop, live lifecycle and discovered
  listeners in the app, with an explicit system-browser preview.
- Existing constraints: repository commands require exact-set approval (ADR 0027/0031).
  Native base predates ScriptEntryView; the node's reviewed 2f67166e introduces it and
  private kind/port. No allocator or health API is assumed.
- Alternatives: a second command runner; local shell or HTTP proxy; reuse node supervision.
- Choice: modify existing workspace detail; pure rules in view/workspace/scripts,
  typed calls with deadlines in link/workspace/scripts; drawing/actions in
  app/areas/workspace/scripts. Existing approval continues to own authorization.
- Why: one supervised process owner and one approval policy; no added runtime/dependency.
- Advantages, costs and effects: closing the app leaves services running. Visible detail
  refreshes at a named interval; late replies must match machine/project/read generation.
  Writes never retry automatically. Failed reads disable actions until refreshed.

## Deliberate differences

- Initial entry point is Settings > Projects > project, where Vessel's existing approval
  lives, rather than Paseo's workspace header popup. Session/worktree-scoped scripts are
  not included in B1's per-project surface.
- Node `script.restart` owns restart semantics; the client does not implement kill/start.
- Node service state is shown literally (starting/running/backoff/stopped), never called
  healthy merely because it runs. There is no health probe on this base.
- Only listeners attributed to this project's script terminal IDs are listed. Port scan
  pending/unavailable/truncated are explicit. No guessed or unrelated process port.
- Browser preview now requires an explicitly supplied host (the former saved-book suggestion was removed under D2 admission fencing); remote loopback or relay
  routes are never silently redirected to the desktop's localhost. Secure/relay/SSH hosts require an explicit host; a remote loopback socket is refused until the person supplies their tunnel host. No proxy, credentials or tunnel is created by this package.
- Browser uses the operator's normal system browser/account after an explicit press;
  only an HTTP URL built from a validated host and nonzero discovered port is accepted.
  Windows/macOS/Linux use GPUI's opener; browser hosts use their platform opener.

## Validation and handover

Focused view and link tests cover run selection, pending/stale replies, terminal ownership,
preview reachability and IPv6, wire requests and refusals. Whole native gate and private
fake-fleet captures once at package end (A14), never the live preview or real agents.
Node field integration must use ScriptEntryView with all fields named (no `..`), and
show port changes in both current approval and last-approved comparison before handover.

## Structure of the first working version

`view::workspace::scripts` owns pure correlation, lifecycle mapping and preview
validation. It reuses `git::Reading` / `RequestKeys` rather than duplicating their
stale-open protection. `link::workspace::scripts` carries only existing protocol-11
calls, bounded by one deadline with no automatic write retry. D2 replaces raw
alias lookup with the shared immutable connection admission and removes the old
`script_host` book reader; there is no admitted-route suggestion until E2 supplies
a reviewed projection. Peer failures expose structured codes without raw strings. `workspace::actor` moves the prior workspace registration
out of the shared link file, reducing its size. `detail_load` likewise isolates
project opening from its already oversized presentation. No crate dependency changes.

The service host field requires an explicit host (including a person's
own local tunnel). The full computed HTTP URL is displayed before Open preview.
The browser callback rechecks the latest terminal, exact listener, successful scan
and ready read; a restart, failed refresh or different PID invalidates the old row.
Service state is never called healthy. Running uses the kit's neutral badge, because
this node has no health report and the shared badge has no running-specific variant.

Entry access is isolated in `scripts::Entry`; Step 0 replaces it with the complete
five-field `ScriptEntryView` and displays declared port separately from listeners.


## Resume point

First implementation is committed as a skeleton; it has not yet passed the native
compile/gate or visual review. Focused view tests passed before the retained-run
regression was added. Next: finish changed-crate clippy/tests, merge the centrally
announced Step 0 base, consume ScriptEntryView when its node change is included,
then one whole native gate, private fake-fleet interaction/capture proof and review.
The private fixture seeder is `scripts/script-fixtures.py`; it leaves approval,
Run, Restart and Stop to the app rather than performing those actions through CLI.
