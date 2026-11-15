# Shell fixture verification and continuation

## Resume point (2026-11-15)

The previous shell worktree belongs to the continuing sidebar slice. This
continuation uses a separate branch and local Cargo target directories; it
does not modify that worktree. The next sidebar work waits for its explicit
handoff note. Theme cycling also waits for the theme consumer's typed API
and review disposition; it must use the existing action registry.

## Paired-book fixture verification

Implementation: `d30804b2133597d1df65ddc71c8cfdb52dd6ced7`, already supplied
by the native integration branch. No second implementation was written.
`scripts/shell-fixtures.py::endpoints` now reads paired `direct` and `relay`
routes as well as a legacy `url`. Each address still has to name the exact
loopback host and avoid the existing protected service ports. An entry with
no address and an adapter without the real-shape fake markers are refused.

[Decision Log]
- Purpose: restore capture-fleet startup after pairing changed the book's
  stored transport from a token URL to key-based routes.
- Existing asset: the fixture already verifies its disposable endpoints
  before creating sessions through the real CLI, then reads the node's
  durable projection. The product book decoder documents both transport
  forms in `crates/vessel-client/src/book/{mod,content}.rs`.
- Alternatives: weakening the guard or reverting pairing would stop testing
  the current client transport. A separate parser dependency is unnecessary.
- Choice: reuse the integration branch's small endpoint projection and
  verify its existing guard and the complete fleet startup independently.
- Effect: no product API, wire field, dependency, UI, or preference changes.
  This is fixture behavior, so no Paseo screen is being reproduced here.

On the exact implementation commit, `scripts/fleet.sh start` completed with
exit 0 and its final `fleet up` marker. This included the existing settings,
wire, agent, conversation, workspace, question, permission, usage, plan and
queued-prompt checks, followed by the previously failing shell seed.
An independent `shell-fixtures.py check` then exited 0. The laptop database
contained 24 sessions, eight long titles, two pinned sessions and two archived
sessions. All eight shell sessions were created through the CLI.

The CLI, node and fake executables came from a successful source build of
the root workspace used by the earlier capture run. Its root source is
unchanged between `56a7cbd3` and `d30804b2`; no root compilation was repeated.
The private evidence manifest records the four executable hashes and both
fixture script hashes. A separate 20-case guard probe accepted both book
forms and checked remote-address, each protected-port, addressless/SSH and
non-fake-adapter refusal, including unsafe later direct/relay alternatives.
It neither opened a connection nor read a credential.

The fixture ran under the development account in a private scratch home
(mode 0700). Both recorded node PIDs were checked against the executable
and fixture path before stopping them. PID exit notifications completed,
and a subsequent process scan found no owned node or fake harness left.
No service, ownership, permission policy, preview or real agent was changed.

Evidence is external to this public repository: the private run folder's
`fleet-start.log`, `shell-check.log`, `manifest.json`, `cleanup.json`, and
the sibling `guard-probe.json`. No screenshots were taken: this verifies
fleet creation, not screen appearance. The native integration branch's
whole gate on the implementation commit remains its owner's evidence;
this continuation changes documentation only and runs the native fast
format, size, boundary and token checks without repeating compilation.
