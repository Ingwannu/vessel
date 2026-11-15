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

## F: painted rows and cursor frames

[Decision Log]
- Purpose: remove row projection/allocation from a quiet cursor-only frame.
- Constraint: the existing draw key correctly invalidates theme, geometry,
  history viewport and output; the renderer and model cursor remain authoritative.
- Alternatives: rebuild rows at every half-period, cache a second renderer,
  or share the immutable rows between the on/off frames.
- Choice: `GridFrame.rows` is `Rc<[GridRow]>`. A retained unblinked frame is
  keyed by generation, scheme, font, metrics and viewport. Blink is a separate
  draw input, so only the frame's optional cursor changes in a quiet blink.
- Why: the UI-thread-owned frame needs no atomic reference count. Keeping the
  model-derived cursor avoids reimplementing hidden/scrolled cursor rules.
  Opening/closing drops the cached projection; output and appearance replace it.
- Costs: one unblinked frame remains retained alongside the presented frame;
  both share the same row allocation. Painting still visits the visible rows.

Paseo reference reopened: app `terminal-emulator-runtime.ts` 560–561 enables
its blinking bar; exact installed xterm `DomRenderer.ts` 528–573 draws supplied
rows and 664–735 manages cursor CSS/idle state. Vessel shares native painted
rows instead of using DOM/CSS. This is an implementation difference, not a
claim of equivalent idle-stop timing or measured GPU work. The existing native
focus, visibility and reduced-motion lifetime contract remains unchanged.

The first regression commit `9e40fd87` retains on/off/on frames, requires the
same row allocation and cursor restoration, then requires changed text to
replace the allocation. Its old-code remote run is preserved separately from
repair validation. The existing quiet-theme and motion tests remain unchanged.
No new speed claim is made before an isolated, same-source runtime comparison.

## Terminal settings consumer boundary

E2 owns the single persisted Terminal preference, Settings draft/commit page
and words; C2 owns `ScrollbackLines` validated serde, live model retention and
`Shell::terminal_scrollback_changed(limit,cx)`. No duplicate bounds or store.
The setting is retained while replay owns the model; both history and live
handover apply it before drawing. A new opening uses the retained limit.
Lowering the bound evicts oldest rows and advances their origin, preserving a
still-retained viewport anchor and clamping a genuinely evicted one. Increasing
it never restores evicted output. Zero removes history and follows live output.
The UI-independent settings module is `cfg(test)` staged under A10 until
E2 preference wiring removes that attribute in its consumer commit; it is not yet proof that the actual Settings page changes a live terminal.
The shared engine migration must preserve these same retention tests/contracts.


## T2 paste preparation

The pure native paste preparation uses the read Paseo runtime
`terminal-paste.ts` 26–34: raw text without bracketed mode, otherwise wrap once
and remove ESC from every embedded end marker. Clipboard-empty skips sending.
Unicode and newline bytes are preserved. One complete packet is checked against
`SendTerminal::MAX_BYTES` before allocation/queueing, including marker overhead;
this additional refusal prevents a truncated bracket sequence and multi-byte
text bypassing a character-count queue. No clipboard access or runtime input
handler is claimed yet. The single ordered sender remains the delivery owner.

## T2 normalized keyboard encoder

The native encoder consumes closed key/modifier/protocol enums and the existing
`CursorKeys`; it never parses PTY bytes or owns keyboard focus. It follows the
read Paseo protocol `terminal-key-input.ts`, including all F1–F12/navigation
modifiers, Shift-Tab, Ctrl symbols, Alt prefix and Win32-before-Kitty modified
Enter. Committed IME text stays on the literal text path; a non-BMP key string
is not mistaken for a control byte. W22's sole dispatcher retains admission.

A frozen fixture was generated by executing Paseo's actual encoder, source SHA
`2c2fbc3a272cdc1cb9b046cebf41e68104cf4fbbf4276c71cc1b1f7cff1b3cc4`.
It compares 4,352 combinations of names/scalars, four modifier bits, cursor
mode and modified-Enter protocol. This is byte-codec evidence only; actual
GPUI key input, IME, node delivery and physical keyboards remain unverified.
No new external API or dependency is needed for this reuse. The shared screen
engine choice is still pending its independent two-candidate comparison.

## Bound history read security cutover

[Decision Log]
- Purpose: a saved alias can be replaced after navigation authenticates a node;
  terminal history must remain with the original destination.
- Existing asset: checked W21 `f3c7db5f` provides required `Binding` and
  `Access::ReadTerminal`, validates the live NodeClient synchronously and captures
  that exact client before spawning. The old terminal area bypassed this path.
- Alternatives: recover authority from current Fleet, trust a bare alias, or carry
  the admitted binding/opening through the existing command. Only the last is used.
- Choice: an opaque frontend `Admission` validates host/session metadata before
  draft switching or hydration. It is consumed once to begin the presentation.
  `TerminalOpening` retains expected NodeId and its existing Arc identity; the
  link also requires that node to match Binding before looking up/capturing a client.
  Missing/mismatched bindings produce the shared translated navigation refusal.
- Why: one owner performs admission without a second connection cache or pane
  manager. Refused navigation preserves the previously admitted screen/draft.
- Cost and limits: this repairs passive history reads. It does not establish
  G4 live lease/resident-stream safety. Legacy Records/SessionLost still carry
  only SessionId; queued cross-node stream fencing was reported to W21 separately.

The two original app refusal regressions actually fail at line head `20b24af24`
(0 passed/2 failed, `/tmp/work/m12-conv-c2/binding-line-red.log`). Extra regressions
cover retained expected node, preserving an existing opening/draft on refusal,
matching node/binding in link, and a read accepted on A still using A when the
saved alias is replaced by B before the asynchronous task runs. Peer RPC tests
use the real typed NodeClient over in-memory framed transport, no process/agent.
No transport/hardware/whole-D1 pass is inferred. Per-commit security review is
owed after exact changed-source narrow checks.

The bound-history changed-package strict check caught `get_unwrap` in the
keyboard oracle test, before a green report. Direct test indexing replaces
that spelling; fixture values and every assertion remain unchanged. The
failed log is preserved and a changed-source check is still owed.

The next strict check reached the app and caught a missing statement semicolon
in the bound opening arm. The spelling is corrected; admission, navigation and
the retained binding do not change. This source remains isolated until the
shared connection/projection adaptation lands on the package line.
