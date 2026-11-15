# Shell fixture verification and continuation

## Resume point (2026-11-15)

The previous shell worktree belongs to the continuing sidebar slice. This
continuation uses a separate branch and local Cargo target directories; it
does not modify that worktree. The sidebar handoff has now been read and
its completed source integrated at `22025b12`. Width captures are complete,
with the historical release's layout failures below retained. The separate
sidebar-height package `0690faed` received bounded source/gate/inspected-height
visual approval; it is not mixed into this branch.
This branch now addresses the theme-cycle input-boundary return through the
existing action registry. The producer's bounded source/native gate passed;
actual native terminal focus and complete platform input behavior remain
outside that evidence. The next package is the sidebar part of F4; command
center and keyboard ownership transfers to its package owner after this action.

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

The first run used the CLI, node and fakes from a successful source build
of the root workspace used by the earlier capture run. Its root source is
unchanged between `56a7cbd3` and `d30804b2`. A second run built all four
executables in this continuation's own Cargo target directory, using
`cargo build -p vessel-cli -p vessel-node -p vessel-testkit --bins` through
the build-slot wrapper. The build succeeded in 2m41s. This second fresh
fleet also completed with exit 0, passed the independent shell check and
produced the same four session counts. In particular, the wire fixture's
`home role missing` assertion did not fail: its home/member/standalone,
join/leave/sync and read-only checks all completed. No fixture or CLI change
was needed. The reported failure with another earlier binary is not
reproduced here; its underlying cause has not been established.

Each private evidence manifest records the four executable hashes and both
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
format, size, boundary and token checks without repeating that whole gate.

## Sidebar width evidence

Completed source inherited, not redrawn: row menu `f7e91461`, ended-session
default `8366a2b4`, projects unfolding `d33d1ba6`, token correction
`4bcdc117`. The last source gate was native 998 passed, zero failed, one
ignored. Integration `22025b12` combines this with `d30804b2`; its only
manual conflict resolution preserves the removal of the three obsolete
sidebar token exceptions. Its complete remote native gate passed every
step: 1169 tests passed, zero failed and one ignored. The private run id is
`shell2-20261115T034200Z-22025b12`; the source was not changed during it.

The provided release executable was built from `d33d1ba6`, SHA-256
`041c0ccc1723b7a1bd8438d656f34dd618b97f6364541585e45c736bdf0bd665`, build
id `97f765a0b55549bbae7826fbe43621bdc16acb74`. The later token correction
replaces equivalent numeric literals with constants and removes exceptions.
These captures are evidence for that sidebar source, not a newly built
integration executable or a whole application visual pass.

The fresh disposable fleet completed the full startup checks with locally
built binaries. Eight additional Git projects on the second node, added
through the real CLI, join three on the first node. Thus the eleven-project
count exercises both machines. No old fleet or capture driver's state was
modified: the driver, preferences, per-run books, display cookies and output
folders are separate copies.

All 86 original screenshots were opened and inspected; the private manifest
records each actual size and SHA-256. Coverage: stored sidebar widths 200
and 600 at 1280×800, machine/status grouping, dark English; narrow sidebar
in light Korean; both stored limits at 800×600; unfolding, refolding and
the light Korean context menu at both limits. The remaining theme/language
cross-product, phone, additional scales and 1024-wide captures were not
rerun in this supplement. Earlier board-agreement evidence belongs to the
original row-menu run and was not independently repeated here.

Reference read: Paseo `components/sidebar/sidebar-workspace-menu.tsx`,
including action order, hover trigger, context surface and its width and
alignment. The existing native omissions and differences documented beside
`sidebar/row_menu.rs` remain; this verification does not approve them away.

Works in the captured states:
- At the wide window both stored widths are drawn; machine names, row menu
  dots and the final more row stay within the sidebar. Long titles use an
  ellipsis. The ended group is visible with the preference field omitted.
- All projects unfolds the eleven projects in place, including the other
  machine's rows. Refolding restores the previous session list and foot.
- The Korean menu's four items are readable, and Escape closes it.

Does not work:
- At 800×600, the first eight project rows already push sessions, Add machine
  and the settings foot below the window. The scripted wheel actions do not
  recover them. The requested width 600 is correctly constrained to 400 by
  available room; the vertical failure occurs at either width.
- At 1280×800, expanding all eleven projects also removes the session list
  and settings foot from view. Show fewer is reachable and restores them,
  but the expanded layout is not a passing state.
- Status grouping at width 200 preserves the machine name at the cost of
  leaving almost only an ellipsis for some titles. This is a readability
  limitation even where elements do not overlap.

The first capture batch was interrupted with exit 143 after two complete
combinations; its cause was not established. Those originals and the failure
record were retained. A detached driver completed only the remaining cases.
The abandoned display PID was absent before its owned stale markers were
removed. Later app/display processes ended normally. That disposable fleet was stopped after the width run, with its recorded
PIDs verified. A new isolated fleet is used for subsequent checks and must
also be stopped by verified PID when the work ends.

## Theme cycle contract and input-boundary return

[Decision Log]
- Purpose: expose Paseo's theme-cycle action through the native app's single
  existing shortcut registry, including rebinding, conflict detection and
  the Settings row that already read that registry.
- Existing assets: `view::shortcuts` owns the action table, Mod convention,
  matching and stored overrides. The app's one interceptor owns dispatch,
  recording and composition suppression. `Theme::next` and the existing
  `PreferenceChange::Theme` setter own ordering and persistence.
- Alternatives: a second listener or a copied theme list would drift from
  the registry and appearance preference. No dependency or wire change is
  needed; this extends ADR 0036's one-source keys rule.
- Choice: one General action with default `Mod+Alt+T`, resolved to Command
  on Mac and Control elsewhere. A pure view rule suppresses this action
  while the command center is open, and while a terminal has focus off Mac.
  Existing recording/composition guards still precede matching. Overrides
  are subject to the same scope rule as the default.
- Identity: the native action id is `cycle-theme`, following the existing
  validated preference grammar. Paseo calls it `theme.cycle`; broadening the
  shared stored-id grammar is unnecessary for this single native action.
- Reference: Paseo `keyboard/keyboard-shortcuts.ts:1028–1050`, General help
  label "Cycle theme"; `app/_layout.tsx:388–391` computes the next preference,
  and `hooks/use-keyboard-shortcuts.ts` routes that callback. The referenced
  context and matching types were read as well.
- Dependency contract: theme consumer `fb711444`, including the correction
  above `4f754b4b`. It adds the eight existing preference variants and
  `Theme::next`. Its owner and review confirmed the bounded source/native
  gate (1181 passed, zero failed, one ignored); that is not a complete theme
  runtime or visual approval.
- Verification: match the real default on both conventions; test command
  center and terminal exclusions, Mac terminal allowance, rebinding and
  the complete cycle through `PreferenceChange::Theme`. Native whole gate
  follows narrow compilation. Actual terminal focus needs the terminal
  area's public seam; the current integration still draws a placeholder,
  so a pure context test must not be described as live terminal proof.

### Host input boundary

The initial registration's native gate passed (1187 passed, zero failed,
one ignored), but review returned its host-input boundary: GPUI still sends
character-preferred events, including Windows AltGr, to interceptors. The
listener now clears a pending sequence and returns before matching when
that preference or focused composition applies. It does not consume the
event, run an action or save a preference on this path. A real GPUI platform
key event with a focused kit input tests the interceptor preference, raw
event propagation, unchanged theme and cleared sequence; separate real key
events exercise the eight-step cycle and palette exclusion. These three checks passed through real GPUI input dispatch, and the changed
app passed narrow Clippy. Actual OS AltGr and Mac
keyboard behavior are not implied by the GPUI probe.

There is no terminal focus surface in this native integration yet. Its
named query explicitly returns false until the terminal area's focus seam
lands, rather than interpreting an invented toolkit context substring. The
view's non-Mac terminal refusal and Mac allowance remain policy tests only.
The existing test-only isolated shell constructor becomes crate-visible to
reuse its no-I/O setup; no production factory or API is added.


### Runtime evidence and package base

The immutable capture executable was built from `4d7ab106` (ELF build ID
`62992542b682e5dbc5d2db4bfd8edd7fdfbdff73`). Nineteen original frames were
opened individually: desktop EN/KO and phone-width EN/KO show the General
row, Light to Dark after Control+Alt+T, and desktop palette-open suppression
followed by Escape and transition to System. Each saved preference projection
matches the visible state, with no custom binding or forced theme variable.
The first desktop KO navigation attempt remained on Draft; those five frames
are retained and excluded from settings-row proof. The corrected desktop KO
set supplies that proof. The capture manifest records each original's hash,
size, role and the limited platform scope. System resolved to light in this
isolated run; this is not proof of OS setting change delivery.

The announced package base `8d5513e4` was merged cleanly at `961e55b4` before
the final package gate. Shortcut, policy and theme product sources are unchanged
from the capture source; the inherited isolated test factory now constructs the
base's extracted update state. Captures are evidence for that exact executable,
not a claim that the newly combined application's other areas were inspected.
The one final combined native gate and review follow this checkpoint.
