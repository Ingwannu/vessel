# M12: the effort slider as the Codex app has it (items 2 and 3, 0.0.0.0.13)

Owner, 2026-11-14 (words 20, 21, 23, 24): the effort slider looks low
quality; learn the local Codex app's slider; smooth movement that "sticks"
to an effort; make the effort's name open the detail settings too; learn
the app's colour change at the top effort and with Fast, and its feeling
of speeding up.

Reference: the Codex desktop app 26.928.20755 on the development machine,
read as data (never executed, nothing copied into the repository). The
reader's notes, with every value's place in the extract, are outside the
repository; the values used here are restated below so this plan stands
alone. Where the notes say "not found", Vessel's own tokens are used and
named as differences.

## What the reference does (values)

- Discrete: the value is only ever a stop index. The pointer maps over the
  whole control width, `round((x - left) / width * (N - 1))`; the chosen
  stop changes as soon as the pointer crosses a midpoint. No pull, no
  hysteresis.
- The thumb and the fill follow an animated position (percent of the
  track). Each change of stop tweens there: 150 ms while dragging, 300 ms
  for a click, a key or a wheel step; easing `cubic-bezier(.23, 1, .32, 1)`.
  Reduced motion: the position jumps.
- During a drag the effort's word and the selection change live; on
  release the choice is sent only if the stop differs from the stop at
  pointer-down. Keys and wheel steps send at once.
- Thumb: 28 round, white, 0.5 border (`border-strong`), shadow
  `0 0 2px #0000001a`; grows to 32 while hovered or pressed with a spring
  (stiffness 420, damping 38, mass 1), back to 28 with (220, 26, 1).
- Track: 24 tall, radius 12, ink at 10 %, an inset 0.5 border; the control
  28 tall inside a 32-tall container (padding 2 / 6, margin 2).
- Fill: the accent (the reference's default `#339cff`; Vessel uses its
  accent token, the one named difference), its right edge under the
  thumb's centre.
- Stops: 4 across; stop i of N at `left: P% + (13 - P / 50 * 13) px`,
  `P = i / (N - 1) * 100`; passed stops `#ffffff4d`, others ink at about
  35 %; a hovered stop scales to 2 (hit area 16).
- Wheel: one stop per 30 units, the count reset after 160 ms or a change
  of direction. Arrow keys: previous / next stop.
- The top stop (the agent's `ultra` / `max` kind, decided in `view`):
  the effort's word, the thumb's labels and an "on" bolt turn purple
  (dark `#ad7bf9`, light `#924ff7`); the fill becomes a moving purple
  field (the app: a WebGL gradient of 12 moving colour blobs,
  `mix-blend-mode: luminosity`, revealed by a 2 s mask); a 16-particle
  burst at the thumb on arrival (0.62 s, `cubic-bezier(.25, 1, .5, 1)`,
  5 px particles, about 34 px of travel); while dragged to the top, the
  warning "Consumes usage limits faster" covers the title row, purple
  with a 1.1 s shimmer, and the bolt, title and reset are hidden; the
  stops fade over 0.4 s.
- Fast on: the bolt in the accent (filled glyph); 14 white dots (3 px,
  `#ffffffb8`, glow `0 0 5px #ffffff57`) drift along the fill, 1.9 s per
  pass, linear; the stops slide -85 px, shrink to 0.75 and fade
  (0.8 s / 1.15 s / 0.5 s).
- The card's title row: the model's name, the effort's word and the
  chevron are one button (min 32 tall, padding 4, radius 8, inner radius
  6, gap 2) that opens the model list; the bolt (32 × 32, left) and Reset
  (32 × 32, right) are separate neighbours. Fast's tooltip: "1.5x speed" /
  "More usage" (second line at 65 %).
- Exact numbers for the field, the burst and the dots are to be read in
  the extract's slider script (offsets 600-9000) and stylesheet
  (605-5000) before painting them.

## Plan

1. `view`: the slider's logic, under test: stop ↔ position (the inset
   rule), the stop under a pointer, the tween (the cubic-bezier, by
   Newton steps), the thumb's spring (semi-implicit Euler at the frame
   step), the wheel accumulator, "commit only if changed", and which stop
   is the top kind (`ultra` / `max` value ids or the folded top switch:
   one table, a third agent is one line). Fast is "an option of the fast
   role" in the same table (the coordinator, 01:35: Codex-only options
   such as a multi-tier speed or `daybreak` may come; the rows are
   written against roles, not today's two ids).
2. `kit/src/slider.rs`: rebuilt to the values above; motion from the
   frame clock only while the card is drawn and something moves (measure:
   no frame requests with the card closed); reduced motion jumps and
   draws no drift, burst or moving field, colours still change.
3. The painted field: GPUI has no WebGL or blend modes. The app's field
   is a weighted mean of 12 colours at Gaussian blobs
   (`exp(-12 d^2)`) whose centres move on sines (speed 1.25, each its own
   frequencies, each weight pulsing 0.7-1.0), mixed 4 % toward its dark
   violet, a 0.9 gamma and a little grain; it is cheap enough to compute
   on the CPU at the fill's device size each frame, painted as one image
   (the previous frame's image dropped from the atlas). The 2 s reveal is
   a mask: a 120 px soft edge sweeping from the right end to the left.
   Every difference from the app (the luminosity blend above all) is
   named in the control's Decision Log. Judged by frame strips: every
   100 ms for 3 s at the top stop and with Fast on, both themes.
   Burst: 16 particles from 0.25 to 1.28 scale at 22 %, then to 0.55 at
   their own offsets (about 34 px out), staggered 0-14 ms, every fourth
   mixed 18-32 % toward blue. Fast dots: 14, each pass 1.9 s scaled by
   up to ±20 % per dot, rows spread over 12-88 % of the track's height,
   opacity 0.4-1.0, scale 0.5-0.95, fading in over the first 8 % and out
   over the last 8 % of the pass.
4. The card (`app/src/areas/draft/effort_card.rs`): the title block as
   one target with one hover (item 3); the bolt as the reference's
   32 × 32 button, accent when on, purple at the top stop; the warning
   row; the composer pill checked (name, agent and effort already one
   target there, or say not).
5. Captures: both themes, desktop and phone, English and Korean; a strip
   per animated state; the whole native gate; then the build commit for
   0.0.0.0.13 with C's `e2851a48` / `942b3aa8` merged (the duplicate
   ended line removed from `resume_note`, the composer kept read-only
   when `conversation_cannot_resume`) and Sync hidden unless its fix is
   captured on standalone, home, member and protocol 11.

Item 4 after the cut: the Fast switch row at the top of the detail list
(the same state as the bolt). Daybreak is not drawn: the node advertises
nothing for it (ADR 0055); if a patched adapter comes, the option is
`daybreak` with `off` / `blue` / `red`.

## Status (2026-11-14)

Built: `view::slider` (geometry, motion, flair, state; tests),
`view::effort::{ROLES, Effort::maximum}`, `kit::slider` (draws a frame,
reports the pointer, wheel and keys; frames only while moving),
`app::areas::draft::effort_card` (the card and its title row). Measured
on the fake fleet (debug build, software Vulkan): the open card at the
top stop costs about two cores; closed, about 1 % (nothing ticks).

Named differences: the fill is Vessel's teal accent (an "on" bolt on the
dark card is the teal mixed halfway to the ink: the teal is 2.0:1 there);
the word below the top stop is the ink; the field is sampled every 2
points and painted as one image (the luminosity blend computed per
sample in sRGB); the warning's shimmer is a colour per character in one
text run; the bolt's tooltip has no dimmed second line; a hovered stop
grows without darkening; no keyboard focus ring on the thumb yet.

## Dragging and frames (0.0.0.0.15, the owner's words 41-42)

The owner, on 0.0.0.0.14 on a real GPU: the slider is not smooth when
moved; Fast's dots and the slider's movement look choppy.

- Frames: the control asks for the display's next frame (GPUI's frame
  callback) for as long as anything moves: the thumb, the fill, Fast's
  dots, the field's reveal and the burst all advance once per frame
  drawn, at the display's rate. The 30-a-second timer the decorations
  ran on until 0.0.0.0.14 is gone. Nothing animates while nothing changes
  (the decorations still come to rest 2 s after the pointer leaves, over
  1.5 s), and held motion (a software renderer, reduced motion, the
  Animations setting off) still draws the end state.
- Dragging, a named difference from the Codex app: there the thumb only
  glides from stop to stop while dragged. Here, while the button is down,
  the thumb follows the pointer frame by frame, with a pull onto each
  stop (`view::slider::drag`): within 8 points of a stop it stays on the
  stop, past that its distance rises along a power curve (1.6) to meet
  the pointer at the midpoint, so it lags leaving a stop and leads
  arriving at the next and never jumps. The effort shown changes when the
  pointer crosses into a stop's half of the line. A drag that begins away
  from the thumb closes the gap with a 60 ms time constant. On release the
  thumb springs onto its stop with a critically damped spring (the Codex
  app's press stiffness 420, damping 2√420): no overshoot. A click on the
  track, the keys and the wheel keep the app's 300 ms glide.
- The stop under a pointer is now the nearest along the line between the
  end stops (where the thumb is drawn), not the share of the whole
  control.
- Tests (`view/src/slider/tests.rs`): a scripted drag at 60 frames a
  second (positions frame by frame, never back, no step over 6 points,
  the word changing at each midpoint), the magnet's reach and continuity,
  the release spring's settle (under 0.3 s to within a quarter point, no
  overshoot over a point), a drag catching up, a click gliding, held
  motion staying on stops.
- Proof without the owner's GPU: `VESSEL_APP_FRAME_LOG=<file>` writes one
  line per frame the slider draws (`kit::frame_log`); the numbers from a
  scripted drag and a Fast run are in the build's report. A virtual
  display on a software renderer shows that a frame is asked for each
  frame drawn and none when still; only the owner's screen shows whether
  the motion reads as smooth at its refresh rate.

## Known costs (2026-11-14, 0.0.0.0.14)

Measured on the release build on a virtual display (lavapipe, a software
renderer; `top` over 5 s), the way the coordinator measured 0.0.0.0.13:

| State | 0.0.0.0.13 | 0.0.0.0.14 |
| --- | --- | --- |
| Card open, middle stop | 6.6 % | 0.4 % |
| Card open, top stop, pointer on it | 517 % | 0.4 % |
| Card open, Fast, pointer on it | 508 % | 0.4 % |
| Board, idle (working spinners) | 530 % | 0.4 % |
| New session, composer focused, idle | about 31 % | about 31 % |

The caret, after the patched gpui-base (0.0.0.0.15; the same release
build settings, the composer focused by arriving on the new-session
screen, the pointer still; two 5 s readings each):

| Window | 0.0.0.0.14 | 0.0.0.0.15 | 0.0.0.0.15 with `VESSEL_APP_REDUCE_MOTION=0` |
| --- | --- | --- | --- |
| 1280 × 800 | 29.1 %, 32.6 % | 0.4 %, 0.4 % | 28.2 %, 29.9 % (the caret blinks) |
| 1920 × 1200 | 64.6 %, 60.2 % | 0.6 %, 0.8 % | |

- On a software renderer the whole app holds its motion still
  (`app::motion::note_renderer`); with `VESSEL_APP_REDUCE_MOTION=0` the
  card's top stop costs 125 % and Fast 222 % while the pointer is on the
  card, 0.6 % once its decorations have come to rest.
- The text caret's blink (gpui-base's `BlinkCursor`, every 500 ms, each
  toggle a whole-window redraw) was a known cost on 0.0.0.0.14. Since
  0.0.0.0.15 gpui-base is a patched copy whose caret is steady while
  motion is held (`spikes/release/patches/README.md`, change 4); with
  motion allowed on a real GPU it blinks as before (to be measured on
  Windows, track R).
- GPUI repaints the whole window on any change (only cached views reuse a
  frame, and the shell is one view): the costs above are bounded by
  asking for fewer frames, not by repainting less.
