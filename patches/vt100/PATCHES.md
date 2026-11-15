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
2. Primary scrollback capacity mutation, retaining newest rows and notifying trims.
3. Primary soft-wrap reflow on resize without replaying synthetic ANSI or resetting
   saved state; alternate resize keeps xterm's clipped-but-retained right cells.

Wide-continuation attributes are corrected by the facade, not another parser.
No byte parser, runtime, PTY, toolkit, or harness is added to the upstream crate.
