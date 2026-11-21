# Scripts in the page workspace

## Decision log

- Purpose. Use the workspace header to list its configured scripts and
  services, run and restart them, view their terminal or stop them. The
  desktop uses an anchored menu and the compact page uses a sheet.
- Existing contract. The active protocol-11 `script.list`, `script.run`,
  `script.restart` and `terminal.kill` calls serve the workspace's owner
  node through the web bridge. Project roots use their project handle.
  Worktrees use the anchored session so the node resolves the exact
  checkout. `project.approve` records the digest shown in the existing
  approval dialog. The node checks the digest again before running.
- Choice. Reuse the live contract and the existing workspace terminal
  tabs. A stopped terminal is kept long enough to read a short script's
  output. Configured rows use the terminal registry's lifecycle when a
  run exists. A removed script can still be stopped while it is live.
  The control refreshes when the workspace's terminal registry changes,
  and while the list is open, because the live script list is unary.
- Staged service contract. The node reports process lifecycle, including
  a service's restart backoff. Health, verified Open and Copy links and
  route choice need a reachable service observation and publication
  contract. The `workspace.script.*` and service exposure methods are
  staged and refused by the web bridge. `port.list` reports a listener
  without establishing a public URL or application health. Follow-up
  coverage is in TRACKER 507 for service rows and 513 for in-app preview.
- Verification. Run the web gate with the real isolated node fixture.
  The browser test covers approval, start, restart and stop. Capture the
  desktop and compact lists in light and dark themes.

## Paseo 0.15.4 reference

- `packages/app/src/screens/workspace/workspace-screen.tsx:3184–3199`
  places the Scripts button in the workspace header.
- `packages/app/src/screens/workspace/scripts/workspace-scripts-button.tsx:312–447,450–515`
  draws the rows, controls and trigger on desktop.
- `packages/app/src/screens/workspace/scripts/workspace-scripts-menu-pages.tsx:77–140,206–345`
  covers compact navigation and actions.
- `packages/app/src/screens/workspace/scripts/use-workspace-script-actions.ts:69–165`
  starts and stops terminal-backed jobs. Vessel's restart runs through
  the node's retained script plan.
