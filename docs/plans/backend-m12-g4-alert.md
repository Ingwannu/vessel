# Shared alert

[Decision Log]
- Purpose: pairing's password warning, failed reads and action refusals
  need Paseo's same notice in every area.
- Existing assets: the kit has a status badge, settings row errors,
  semantic success/destructive/neutral colours and embedded vector
  drawings; it has no alert. A badge is a short label, not a wrapping
  explanation, and has a different variant set.
- Alternatives: a screen-local warning; stretching the badge API; a new
  external component dependency.
- Choice: one kit control with all five reference variants and all four
  density sizes, a separate toolkit-free view enum, one app mapping.
- Why: the reference is the specification, and pure decisions must not
  depend on kit types. Another dependency would not supply these exact
  values or the existing kit's text faces and scaling.
- Impact: two palette tokens and two own-drawn glyphs are added; no new
  dependency, node call, screen or translation string is introduced.

The source comparison is with Paseo `components/ui/alert.tsx`,
`control-geometry.ts`, its `control-geometry.test.ts` alert cases,
`utils/color.ts` and `styles/theme.ts`. The reference has four sizes
(xs/sm/md/lg), despite the initial assignment saying two; the coordinator
confirmed that all four belong in the port. Geometry is exact: horizontal
padding 12/16/24/32, vertical 8/12/16/24, radius 12/16/16/16, icon
12/14/16/20, icon gap 8/12/12/16 and text gap 0/2/2/4. Title and description
use the same kit type step (12 in xs, 14 otherwise), medium title and
normal muted description. The kit's existing line metrics and font
scaling apply, since the reference specifies font size but no line height.

Default has no icon/accent; info uses blue 300; success uses statusSuccess;
warning uses amber 500; error uses the existing destructive colour, not
statusDanger. Default/success use the neutral border; other borders use
exactly 0.5 alpha (`hexColorWithAlpha` emits rgba, not a rounded byte).
The fill is transparent. Body text and actions start at the lead text's
edge; the action has eight points above and can wrap. No text truncates,
no fixed height limits it and the container receives no focus ring.

Info and triangle reuse the kit's own glyphs. Circle check and circle
cross are new, independently drawn paths on the same 24-unit grid with
1.5-point round strokes, embedded by the existing asset source. No path
or asset was copied from Paseo, and no asset licence/dependency is added.

The view enum lives in `view/src/alert.rs`; the kit's independent enum
lives in `kit/src/alert.rs`. `app::badge::alert_variant` is their single
exhaustive adapter beside the existing badge adapter. A consuming area
chooses the meaning in view and supplies localized content and a wired
optional action. Content Debug deliberately omits its text.

Tests check every variant's accent, border and glyph in both themes,
the two new palette values exactly, and all four geometry cases from the
reference tests, including the indent and type step. No dependency on
Devices, its QR, or its work-in-progress state enters this commit.

Validation: the pinned native workspace passed `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings` and `cargo test` as a whole.
The alert-only patch was independently applied to the badge commit in a
scratch index. Visual checks belong to the consuming Devices screen;
this control adds no catalogue route to the app. No installed service,
production permission or runtime configuration changed.


## Partial-host-errors presentation: radius correction

[Decision Log]
- Purpose: the shared messages-only banner must use the reference's actual
  corner size, not the earlier handoff's mistaken interpretation.
- Existing asset: `alert::host_errors` already uses the one shared frame;
  `theme::radius::R2` is 8. No new token or renderer is needed.
- Alternatives: change R5 globally, override a consuming area, or create a
  second banner. Those alter unrelated controls or duplicate the renderer.
- Choice: only HostErrors uses R2; its regression asserts the literal 8
  in both themes. Every ordinary Notice geometry remains unchanged.
- Why: pinned Paseo schedules-screen line 398 uses borderRadius.lg, and
  styles/theme.ts lines 611–619 resolve lg to 8 (xl is 12). The earlier
  producer/handoff and its literal-12 test were wrong; G2 found this and
  V1 returned the radius-only defect before first-use captures.
- Impact: padding 12, gap 4, red 300, normal 12-point type, neutral border,
  wrapping sibling messages and no icon all remain. API is unchanged.

The earlier producer gate (1045 passed, zero failed, one ignored) remains
old-source evidence. It does not validate this correction or the consumer.
Under A14 this correction receives narrow Kit checks; G2's final F1 package
supplies the whole gate and first-use captures. No extra whole producer
run is added. No installed service, production permission or runtime
configuration changed.


Corrected-source narrow results on `7f9c6d19`: remote Kit Clippy exited zero
(run `devices-narrow-20261115T065437Z-7f9c6d19`); remote Kit tests exited
zero, 112 passed / zero failed / zero ignored (run
`devices-narrow-20261115T070735Z-7f9c6d19`). Both ran native-fonts then the
one requested native Cargo command with no skipped step. The Kit was
actually checked and compiled; the exact HostErrors both-theme regression
passed. Evidence folders are `host-errors-radius-remote-clippy-02` and
`host-errors-radius-remote-test-01` in the resumed track evidence. Their
steps-log SHA-256 values are respectively
`b51d6e346c05ae1abacb9df083805997054ff663c176409e79628980a0c984c4` and
`bf25872563171ae129ddde4553ae75f4f46adcf984c29af930fb2c93a2825730`.
The first unregistered submission was interrupted by the build-tool cleanup;
the single retry authorized by its owner is the Clippy run above, not a
second compiler attempt after failure. No consumer screenshot or visual
approval is claimed. The unchanged API is `kit::alert::host_errors`.
