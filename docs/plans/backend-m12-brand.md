# Milestone 12, track B: the brand on screen

The owner's words (2026-11-14): the logo and the branding are "Multiline
Lettering, Multiline Logo"; the new session's composer inside the mark's V
drawn large with the star above it; the same V framing an open session;
the multiline feel stronger, also as circuit traces reduced to parallel
lines; then the window frame and the window controls in the same line
language, and the star travelling to the agent's reply on the first
message, becoming a circle holding the agent's own mark. ADR 0059 records
the decision; `docs/design/brand/README.md`, "Multiline rules", the
measured ratios.

## What was done

1. A study first, drawn over real captures of the app (the lead's build
   against a private fake fleet, both themes, 390, 1280 and 1920 wide):
   three variants (the mark scaled untouched; the mark's loops re-spaced
   for the screen; circuit traces) at two inks, the open session, the
   window frame at three strengths with corners and junctions at scale 1,
   1.25, 1.5 and 2, the window controls in their states, "Vessel" as
   multiline lettering, and the star's journey in eight frames. Text
   contrast was measured where a line passes under text.
2. `view::brand`: the geometry and the motion, tested against the
   captures' measured boxes.
3. `kit::vector`: shapes rasterised at the display's pixels.
4. `app::brand`: the backdrop, the screens' listeners, the journey, the
   avatar.

## Measurements

- The mark: line 19.0–19.5, gap 25.0–25.5 (view-box units), four lines a
  band, loops at (k + ½) pitches, a ring at a line's end (hole one gap,
  outside pitch + line), 60° arms, the star a third of the V's width.
- Text contrast with the drawing (stronger ink): the mark untouched took
  the pickers from 4.58:1 to 3.07:1 and the heading from 16.6 to 11.1
  (dark); the derived drawing leaves every text box as it was (it never
  passes within 12 points of one, checked by `view::brand` tests).
- Lettering: at scale 1 neither multiline version reads at 14–16 px; the
  three-line version reads from about 32 px of cap height (16 at scale 2),
  the loops from about 44–48.
- Window controls: the mark's loops read only from 12 pt at scale 1; at
  9 pt they are mush. The one-line glyphs with the mark's rules read at
  9 pt.

## Problems met (what was run, what happened)

- `xdotool` clicks went nowhere on a virtual display with no window
  manager: the app's window was placed at (322, 140), not at the origin,
  and screen coordinates were used. Reading the window's position with
  `xdotool getwindowgeometry` and offsetting fixed it; typing had worked
  because typed keys go to the focused window.
- The first capture of the 1920 × 1080 new session was black: the app had
  not drawn its first frame after 14 s on the software renderer at that
  size; 20 s was enough.
- The fake fleet's two nodes did not exit on the stop script's SIGTERM
  (a session in it was waiting); they were stopped by their process ids.
- Clippy (pedantic) asked for type aliases for the raster cache's type,
  `#[must_use]` on `vector::inline`, and names longer than one letter in
  the geometry; `float_arithmetic` is denied in this workspace, so each
  geometry file carries one `expect` with its reason, as the slider's do.

## Open

- The session's listeners and the avatar are worker C's to mount (the
  conversation is being rebuilt as Paseo's message exchange); until then
  the frame and the avatar show only in this track's local captures.
- The avatar's place in a reply, and the slot that tells the journey where
  the coming reply's head is, are to be agreed with worker C; until then
  the star travels to the column's left at the conversation's first rows.
- The window frame (two lines recommended) and the window controls (one
  line with the mark's rules, 9 pt) are drawn in the study; their kit
  controls and the lead's mounting lines are next.
