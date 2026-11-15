# Activity ring opacity at the native compositing edge

[Decision Log]
- Purpose and intent: preserve RNWeb ActivityIndicator's actual20% track alpha,
  not merely the same number in an SVG that is later transformed.
- Existing implementation and constraints: the kit uses GPUI's monochrome SVG
  renderer. The initially source-reviewed combined vector encoded track opacity
  inside its alpha mask. The existing legacy Spinner and its callers must stay.
- Alternatives considered: compensate with a different SVG opacity, alter the
  shared GPU shader, use dynamic full-colour images, or split opaque masks.
- Choice: original registered ActivityTrack/ActivityArc masks, with track
  element opacity20% and the foreground's existing750ms rotation path.
- Why: element opacity multiplies tint after the text-mask coverage correction,
  while SVG opacity becomes coverage before that correction. A compensating
  magic number would depend on foreground colour and backend settings.
- Costs and effects: one additional SVG draw per activity indicator. No new
  dependency, area renderer, global gamma setting or legacy API change.

Reference: Paseo `components/ui/loading-spinner.tsx` delegates to installed
RNWeb `ActivityIndicator`:32-unit view box, radius14, stroke4, base alpha.2,
foreground arc20 units,750ms linear, small20/large36/numeric sizes.

## Observed mismatch

The frozen `ef91b954` patched probe used actual colour tokens. At an interior
base-ring sample, background/foreground/base were respectively:

| Theme | Background | Foreground | Captured base |
| --- | --- | --- | --- |
| Dark |24,27,26|161,165,164|72,75,74|
| Light |255,255,255|113,113,122|206,206,209|

Those values differ from ordinary20% source-over compositing in the capture's
sRGB channels. The foreground arc did reach the full requested foreground.
The old alpha fixture tested resvg output before GPUI compositing; it could
not catch this native rendering effect and is not presented as a runtime pass.

Pinned GPUI0.3.10 `svg_renderer.rs::render_alpha_mask` extracts SVG alpha into
a monochrome mask. `window.rs::paint_svg` combines element opacity with tint.
The WGPU `fs_mono_sprite` shader applies contrast/gamma correction to mask
samples before `blend_color` multiplies tint alpha. Opaque interior mask
samples remain1 through that correction; element alpha therefore stays.2.

The new regression checks actual registered opaque track/arc raster masks
and track element opacity in both themes. The combined vector's existing
tests, legacy Spinner vector/period and existing callers remain unchanged.
The previous88 shared-control captures and16 activity captures remain tied
to the old binary; the activity alpha is explicitly a finding in that set.

## Verification status

The source correction is prepared. Its narrow/whole gates and new native
pixel/phase/frame evidence are still owed. No corrected runtime result or
UI approval is claimed. Existing App reduce_motion observations are scoped
to the probe's App flag and its prepaint counter, not OS accessibility.

The first narrow Kit all-targets Clippy run failed on two direct f32 comparisons
in the new view-box test. Those assertions now compare exact bit patterns;
no tolerance, lint allowance or production value changed. The failed log is
retained. The corrected narrow run passed with actual Kit checking. The
correction still needs its own A13 whole gate and post-correction pixels;
the preceding ef91 gate cannot stand in for changed Rust.

## Completed source gate and scoped native evidence

The exact `e2827533` run `schedules-20261115T031952Z-e2827533`
completed with six native steps zero and no skip. Ten raw test summaries total
1076 ordinary plus2 documentation tests passed, zero failed, one inherited
ignored. Kit and App were actually checked/compiled, including the new opaque
mask regression. The changed native source, not the preceding ef91 result,
is what that gate checked.

A separate locally built external probe actually compiled Kit/App. Its product
source manifest contains714 files, aggregate
`bcd496ebdec3fff53e6301b4d447d46b4db674bee749c7ebe0778ce105fb7bd0`.
The source archive, external probe/patch, binary and original/proof-family font
hashes are pinned in its build record. No installable fixture path was added.

All16 corrected activity PNGs were personally opened: English1280/Korean390,
both themes, active phases and App reduce_motion static phases. Interior
large36 ring samples now exactly match source-over20%: dark51,55,54 and
light227,227,228; full arc and clear centre match the requested foreground and
background. Measured10/16/20/36 bounds remain correct. Active phase pairs
change; static pairs have identical pixels and settled prepaint counts
111/131/85/120. Four additional personally opened menu pictures show the
corrected Numeric16 pending slot refusing stale Enter while remaining open.
They are a separate final evidence folder, with no reused old-alpha PNG.

Final per-file manifests pin both corrected sets and the separate earlier104
pictures. Exact external tools are retained; the pixel/phase verifier recomputes
read-only. All owned probe processes are stopped. The private virtual display
was stopped by verified PID/executable/start time and its authority file removed.
Only temporary evidence write bits were removed; no service permissions changed.

Submitted to V1 first. This is author-inspected Linux/App-flag evidence, not
independent UI approval, an optical750ms measurement, four-scale proof, other
backend/OS preference integration, keyboard focus restoration or a product
list/Git/provider first-use claim. The after-close key-routing issue remains
open separately. Unchanged Rust does not need another gate for these notes.

## Independent limited disposition

V1 subsequently passed exact e2827533 source, A13 gate and the specified Linux
probe's large36 opaque interior alpha, visible10/16/20/36 phase and App-flag
static state. The reviewer independently opened all20 corrected originals and
recomputed their pinned source/evidence. This supersedes only the pending
disposition above for that explicit scope. AA edges, optical rotation period,
OS preference integration, four scales, other backends, menu focus restoration
and every product first use remain unverified or held as already recorded.
The later0.19 integration and standalone loader producer require their own gates.
