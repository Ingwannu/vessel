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
   avatar. The journey steps once per presented frame and reads the kit's
   motion tokens (an app test pins them); held motion shows its end.
5. `kit::window_glyph`: the title bar's four glyphs as one line with the
   mark's rules, on whole device pixels at every scale
   (`vector::snapped`), with the audit sheet `kit/examples/window_glyphs.rs`.
6. `kit::frame::panel_edges`: the content panel's edge as two lines one
   pitch apart (one at a phone's width), GPUI quads on whole device pixels.

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
- Cost (the lead's method: release build, software renderer, 1280 × 800
  at scale 1, `top` over 5 s after 25 s still; compared with the lead's
  build without the drawing, two runs each): CPU at rest 0.4–0.8 % with
  the drawing and 0.4–0.6 % without, on the new session and the open
  session alike, also right after a journey that ran (motion kept on).
  Memory: new session 192–195 MB against 188–190 (one raster); open
  session 228–230 MB against 213–215 (the frame's raster, and the
  avatars the capture build adds to every reply), the same after a
  journey that ran.
- The journey, captured 20 times slower: the move (the V opening into
  the frame, the star travelling and shrinking), star to circle, then
  the ring and the agent's mark, in order, both themes. While the arms
  move they cross the conversation's text for about the first third of
  the move (about 60 ms at full speed, at the quiet ink); at rest they
  never do.

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
- At 1280 the new session drew the narrow figure: the heading and the
  pickers reported the column's whole width (a flex column stretches its
  children), so no arm could clear them. Both now take their own width
  (`self_start`), and the wide V is drawn.
- The first window glyphs (one 9 × 9 SVG through `svg()`) were grey at
  1.25 and 1.5 (the lead's capture): a one-unit stroke straddles two
  device rows. The glyphs are now drawn in device pixels from their own
  snapped origin, the line `⌊scale + ¼⌋` pixels.
- The first cost measurement showed the open session 75 MB above the
  build without the drawing. The first guess (twelve kept journey frames)
  was wrong: keeping at most two large rasters changed nothing (289 MB
  again), and the software renderer holds motion still, so no journey had
  run. The cause was the raster's box: the shapes' whole extent, and the
  open session's frame meets its apex far below the screen. The backdrop
  now rasterises only the area it fills (228–230 MB).
- A test run from an earlier gate kept going after its clippy failed and
  wrote into the next gate's log; each gate now runs as one chain that
  stops at the first failure and writes fresh logs.
- One app launch on the virtual display failed with "failed to connect to
  X server: Unknown connection error" while the server was up; the same
  command a minute later worked.
- On the first run (no machine) the new session shows a note under the
  pickers ("머신이 없습니다. 먼저 머신을 추가하세요."); the drawing measured only
  the heading, the card and the pickers, so at 800 × 600 and 390 the V's
  bottom passed under that note. Every child after the pickers now counts
  with them.
- At 2560 × 1440 the open session's frame, leaning in as far as the margin
  allowed, reached about 28° and met the composer's corners; the owner's
  sketch draws 18–20° ending in the margin. The lean is capped at 20°
  (1920 × 1080 stays at its margin's 19°).

## Open

- The session's listeners and the avatar are worker C's to mount (the
  conversation is being rebuilt as Paseo's message exchange); until then
  the frame and the avatar show only in this track's local captures.
- Agreed with worker C: the avatar sits inside the column (a 24-point
  slot, 8 points, the bubble), its circle's top at the bubble's top, on a
  turn's first reply only; the waiting row gives `reply_head()` so the
  star lands where the reply will start. Until C mounts it the star
  travels to the column's left at the conversation's first rows.
- The window glyphs and the panel's two-line edge are kit controls; the
  lead mounts them in the chrome and the shell, with 4 points of margin
  above the panel and left of it while the sidebar shows (the outer line
  sits one pitch outside). Once the sidebar's resize handle (owner's word
  29: a 1-pixel line at 25 % ink on the frame's left edge while hovered)
  is in, a capture checks that it and the outer line do not read as one.
- Whether the arms should fade while they cross the conversation during
  the move is the owner's to judge from the frame strip.
