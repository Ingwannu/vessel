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

Numbered patch list (implemented; consumer qualification remains separate):
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
6. Restore upstream's exact `.rustfmt.toml` from the archive's Git revision
   (the published crate omits it). Root `cargo fmt --all` visits this path
   dependency. Its edition/width settings preserve the original twelve
   untouched archive members; only already-owned additions are formatted.
   UPSTREAM-FORMAT-SHA256 records the exact source URL and checksum.

Wide-continuation attributes are corrected by the facade, not another parser.
No byte parser, runtime, PTY, toolkit, or harness is added to the upstream crate.

7. Read-only decoded CSI/control/ESC callbacks and single-byte boundary bridge
   to the same patched VTE dependency. Why: one pure facade tracks negotiated
   modes and presentation without a scanner or second terminal parser.

8. Bounded physical marker tables inside each grid and actual reset callbacks.
   Why: inspected rows follow the same trim/IL/DL/reflow edits without per-row
   identity allocation or an unbounded remap log. Reflow returns physical-row
   marker mappings while cell construction remains unchanged. Region scrolling
   preserves marker coordinates as the pinned reference does; row contents may
   change. Marker creation/replacement validates before retiring prior state.

9. Expose retained-row outgoing wrap metadata. Why: authoritative frames must
    preserve hard/soft line boundaries for selection and reflow without a second
    row tracker, mutable reads or synthesized ANSI restoration.

10. Preserve independent program cursor blink overrides from the same decoded
    DECSCUSR callback (including explicit u32 parameters), clear overrides on
    DECSTR, and inherit the primary cursor on1049 alternate entry. Why: the
    pinned headless metadata oracle exposed lost blink-only state and a wrong
    alternate starting position. Soft reset resets modes/saved presentation
    without erasing live cells. Cursor restore clamps pending wrap like the
    reference's explicit cursor move. No raw scanner or second interpretation added.

11. Make redundant1049 activation non-destructive while retaining DECSC on
    the active buffer. Save cursor attributes separately for primary/alternate
    buffers, including active-buffer-only DECSTR save reset. Why: the pinned
    BufferSet refuses to clear an already active alternate screen and saves
    cursor attributes per buffer. Re-entry must retain text/markers and cannot
    overwrite the primary's saved ink before its later restore.

12. Share actual buffer transitions for47/1047/1049, carrying physical cursor
    coordinates in both directions. Only1049 performs cursor save/restore.
    Why: the reference implements all three through BufferSet transitions;
   47 lacked cursor carry,1047 was absent, and alternate exit must immediately
    dispose its markers instead of retaining them until a later re-entry.
