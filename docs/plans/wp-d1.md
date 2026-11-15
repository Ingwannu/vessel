# D1: native terminals

Owner C2. Canonical scope: `refs/notes/paseo-features/PLAN.md` section 7,
area 4 inventory 1–20. A7 timeline belongs to W10. The accepted T1 is
`c66357e8`, V3 queue 122; this package starts from that source and merges
the announced package base `8d5513e4` under A8. T1's exact native gate
`appconv-20261115T060251Z-84b96489` passed 1,155/0/2. That gate does not
verify this base merge or D1. No D1 feature is claimed passed yet.

## Architecture and reuse

[Decision Log]
- Purpose: terminals belong to a workspace and remain resident while panes
  move, focus changes or the network reconnects.
- Existing assets: `view::terminal` already owns the parser, grid, painting,
  viewport, log fold, ordered sender and resize rule. The app's terminal
  area adapts them to the kit's single `terminal_grid` renderer.
- Alternatives considered: another terminal area/parser/renderer, an
  independent pane manager, and carrying shell ids through a session API.
- Choice: modify these assets and keep one rendering path. Layout owns
  placement; terminal state and node calls remain with the terminal owner.
  `SessionId` and `TerminalId` stay distinct. An opening ticket owns reads,
  parsing, sends and the returned state. Count and identity boundaries are
  typed; screen words use the existing localization pipeline.
- Why: the standard library's ownership, existing executor and controls
  cover this boundary. No external UI or terminal library is needed for
  the skeleton. Additional dependencies or node calls need an explicit
  contract and the normal owner/Quality B review first.
- Costs: a resident stream has its own bounded model/queue/focus state;
  reconnect, old completions and closing must not mutate its replacement.

Data flows from the node through typed link commands/events to a resident
terminal. Replay runs on the background executor; view supplies cells and
rules, app converts them, kit draws them. User input takes the ordered
sender back through the existing node boundary. Layout receives typed
targets and owner close acknowledgements, never a copy of terminal state.

The A8 merge preserves the incoming `shell/events.rs` dispatcher. Its
Records arm uses the existing `link_records` helper, which now also calls
the incoming composer-file reconciliation hook. A terminal-only batch
returns before shell invalidation; other records retain ordinary handling.
Incoming conversation language-test receipts explicitly have no terminal
opening. No root code differs from `8d5513e4`, no size allowance is raised,
and head command/event registration stays in the newly split modules.

## Package sequence and state

1. Complete the base integration's narrow checks. Retain a current release
   baseline, then share painted rows across cursor frames (F), test and
   measure it with sidebar animation hidden through the real preference.
2. Skeleton: enumerate every pane/list/create/rename/close/input/find state
   below, typed link commands/events and fixture-rendered states. Compile
   and narrow-check it before filling behavior.
3. Fill input/paste/resize/query duty (T2), workspace shell lifecycle (T3),
   find/scrollback (T4) and the remaining survey rows. Keep small commits,
   with changed-crate tests/clippy, not a whole gate per subfeature.
4. At package end take and inspect captures at 1280×900, 800×600 and
   390×844, both themes and EN/KO; run one exact native gate, root gate if
   own root code changed, then one package review and lead handover.

Skeleton states owed: host disconnected/not ready; renderer not ready;
attaching/live/ended/attach failed/update required; list loading/failure/
ready/reconnecting; create queued/failed/created; rename and destructive
close confirmation; modifiers, paste/upload/cancel; find results/limit;
focused size ownership. These must become enums, not booleans or strings.

| Survey rows | Existing assets / dependency | Remaining work |
| --- | --- | --- |
| 1–5 PTY, worker, protocol, restore, size ownership | node terminal APIs; native parser/grid | inventory current node head and own remaining gaps; no duplicate node parser |
| 6–7 activity and attention | W7 ADR 0089 | consume typed attention/revision; no inference from PTY text |
| 8–10 lifecycle, pane, focus/resize | W12 sole layout; W11 ADR 0082 workspace | resident `TerminalId` state, follow/list, close confirmation, input and ownership |
| 11 paste/drop | sender; W13 files; W3 terminal launch | text/path encoding, bounded upload/cancel and local/remote routing |
| 12 links | W13 file target; desktop external opener | bounded detection, resolution and main/side destination |
| 13–14 emulator/grid | existing parser/grid/kit control | row reuse, key modes, selection/input and settings integration |
| 15–16 find/virtual keys | existing shared controls | typed results, focused shortcuts, compact modifier controls |
| 17 profiles | W3 owns node profile contract/create flows | consume profile API and icon/settings integration |
| 18 CLI and tools | W17 owns CLI/capture; W18 MCP | provide native seams, use their shared contracts |
| 19–20 auto terminals / workspace launch | W3 create; workspace node owner | attach created resource through W12 target; no harness session |

Node changes use a separate owned node-head tree. Pending rows use Quality
B and keep protocol constants unassigned until integrator review. The
app's older node snapshot is not evidence that a current node call is absent.

## Reference and explicit limits

Survey: `area-4-terminals-scripts-browser.md`, inventory 1–20 and its
cross-area connections. Source root: `refs/paseo-latest/packages`.
The key encoder, input-mode tracker, runtime paste and drop path modules
have been read, including kitty/win32 modes and end-marker defusing. The
existing ADR 0073 records scalar/fallback/underline and reply differences.
The T1 viewport pins live-output history at existing geometry; logical
anchor mapping across width reflow remains owed with resize. F and the
rest of D1 are not covered by T1's PASS or its static capture matrix.

Testing/captures use private fake agents only. No live preview, real agent,
production configuration, public push or shared target is part of this
package. Each worktree has its own target under `/tmp/build`.

## Shared base integration

A8 integration now includes package base 4 `3cd0d3fc` (base 3 includes
Quality B and the coordinator's unchanged N2 `c5da5305` race correction).
The shared event dispatch keeps the base's qualified `areas::Redraw` arm;
the only merge conflict was its equivalent imported spelling. Terminal-only
record redraw behavior and the new base's composer reconciliation remain.
The pre-integration base 2 release is retained as a historical baseline;
it has no runtime measurement or current-base compatibility claim.

The coordinator's ADR 0095 ruling supersedes the earlier assumption that
native view remains the parser owner: a new pure `vessel-terminal-screen`
will own the sole interpretation for native view and node capture. Engine
selection is pending the frozen actual-program corpus measurement against
Paseo's exact server factory. W17's staged vt100 adapter is measurement input,
not a reviewed or merged implementation. No D1/core feature is passed yet.
