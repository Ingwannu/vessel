# Shared action-trigger profiles and passive layout

[Decision Log]
- Purpose: supply G3 ProviderRow's menu trigger and G1's stable Some/None slot.
- Existing constraint: Schedule and GitHeader values/callers stay; one frame
  owns visual/hit extents, anchor and SVG ink, rather than one renderer per area.
- Alternatives: another provider button, area26/32 constants or invisible triggers.
- Choice: ProviderRow plus typed Closed/Open presentation, a common frame and
  read-only visual size/passive Div. Existing trigger/trigger_with defaultClosed.
- Why: no duplicated active shell or drifting layout footprint; raw icon/default
  button geometry is not substituted for this reference-specific control.
- Costs: consumers keep keyed-target/authority guards and attach no disabled
  handler; actual hit/parent/neighbor/clip and first-use UI are not unit proof.

Pinned1c00b365 screens/settings/providers-section.tsx99–171 full
ProviderActionsMenu,205–300 relevant row,605–618 styles; theme.lg is8. Source
MoreHorizontal is the existing Glyph::More, not a new alias. Provider visual32
in both layouts/r8/outside8 =>hit48; hover OR open changes surface2 and foreground;
pressed refines backgroundsurface3 only. A closed touch press therefore keeps
muted icon ink until open. Removing disables, with no invented opacity.

Schedule remains visual22/44+hit38/60/glyph14/r4/padding4 and its existing hover;
Git remains visual26/32+hit42/48/glyph16/r6/static muted with no open wash. The
new common frame retains inner visual bounds as anchor and negative-margin hit
compensation. Only Provider's pressed visual needs a scoped stateful child;
legacy anonymous visual children stay as they were. Pinned GPUI div.rs3497/3574
refines group_active after group_hover, so source pressed precedence is explicit.

trigger_visual_size(profile,sizing) reads that same frame. trigger_space is
flex-none+size only, no identity, anchor, event, opacity or glyph. Real-window
Some/None probes compare countX and row height at both sizes/themes/profiles;
these do not replace G1's actual production file-header/root-mismatch captures.
Old literal tests are kept; new source geometry, palette/default paint and
horizontal glyph mapping have both-theme cases. Changed-crate narrow and source
review are owed. No runtime phase/hit/OS/capture or final package pass is claimed.

Exact isolated6ece4a76 changed-Kit suite is now green: Clippy0 and164/0/0 tests,
fonts/cargo both0/no skip. Runs schedules-menu-20261115T105950Z-6ece4a76 and
schedules-menu-20261115T110007Z-6ece4a76. Changed Kit checking/compiling verified.
This is headless GPUI coverage, not an actual Linux/OS/consumer UI or full App/F1
pass; old native F5, focused TextField, hit/clip/neighbor and first-use evidence
remain owed. V1 source disposition is pending. Coordinator assigned S2 the
additive hierarchy writer AFTER this delta's checked source handoff; no second
engine or concurrent draw writer. The temporary label retires after handoff.
