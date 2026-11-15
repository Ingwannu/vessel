# Vessel's vt100 0.18.1 patches (ADR 0095)

Upstream: https://github.com/doy/vt100-rust, crates.io vt100 0.18.1, MIT.
The exact .crate SHA256 is in UPSTREAM-CRATE-SHA256; every unmodified archive
member is recorded in UPSTREAM-SHA256SUMS. LICENSE is retained verbatim.

[Decision Log]
- Purpose: one maintained screen engine for node capture and native restore.
- Constraint: upstream public API lacks retention capacity mutation and reflow;
  synthetic ANSI restoration would destroy saved cursor/region/mode state.
- Choice: vendor the exact crate, add only the approved grid/retention/reflow
  capabilities and expose a typed facade from vessel-terminal-screen.
- Why: the coordinator's measured ruling selects vt100; the native parser is
  retired only after the new engine's corpus and native-contract cases pass.
- Cost: local patches require independent review against upstream on upgrades.

Numbered patch list (implementation in progress):
1. Read-only full-buffer cell/row access and latent right columns for capture.
2. Primary scrollback capacity mutation, retaining newest rows.
3. Primary soft-wrap reflow on resize without replaying synthetic ANSI or resetting
   saved state; alternate resize keeps xterm's clipped-but-retained right cells.
   Height changes precede width processing; narrowing groups are handled from
   the bottom with current viewport and saved-cursor accounting, including
   output below the cursor. The independent saved-cursor golden exposed the
   first implementation's static-cursor defect; its failing result is retained.
4. Preserve the space-intermediate identifier on unhandled CSI callbacks so
   an unknown sequence cannot be mistaken for an admitted DA/DSR query. Keep
   bold and dim independent until SGR22 clears intensity, matching xterm's
   separate bits and the existing native attribute contract. Independent
   server-handler/raw-cell golden cases cover both corrections.
5. A wide glyph wrapping with one padding cell still marks a soft-wrapped
   logical line. Upstream deliberately used tmux's unlinked behavior here;
   xterm links it. The independent wide-padding shrink/grow golden catches
   the extra history row caused by losing that link.

Wide-continuation attributes are corrected by the facade, not another parser.
No byte parser, runtime, PTY, toolkit, or harness is added to the upstream crate.
