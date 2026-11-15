# Shared button content, loading and measured sizing

Coordinator assigned G2 the sole renderer writer, before the remaining F1 work.
Three control-only changes are ordered: typed content, loading content, measured
size parity. Consumers are S2 settings reorder and G3 provider catalog Add.

Pinned Paseo1c00b365: components/ui/button.tsx (whole), control-geometry.ts (whole),
components/ui/control-size.tsx, components/provider-catalog-list.tsx (whole), and
screens/settings/sidebar/sidebar-nav-section.tsx (whole). MD is pointer32/px16
or touch44/px24, glyph14. SM is28/px12 in both layouts, glyph12. Icon-only width
is the chosen height and horizontal padding0. Catalog Add explicitly chooses SM;
its override is only flexShrink0. Sidebar reorder uses MD with an explicit muted
14-point React element, so its ink stays muted even when Ghost text hovers.
This corrects the earlier ledger's assumed touch32 SM value.

[Decision Log]
- Purpose: one shared label/icon/loading button, without empty text or area divs.
- Existing constraints: legacy label entries and all filled/Ghost paint states
  retain their signatures/defaults; icons need their own SVG ink on this GPUI.
- Alternatives: toolbar pills, area arrows/spinners, or another button renderer.
- Choice: Label/Icon/IconAndLabel with automatic or explicitly muted icon ink;
  extract the existing typed renderer to widgets/button.rs before it grows.
- Why: one paint and geometry path preserves the accepted pointer/touch model.
- Costs: compatibility label-only calls without a Window still default Pointer;
  ownership of their density migration remains outside this control delta.

Step1 is a source checkpoint. Existing caller invocations are unchanged; the
legacy label entry delegates to Label in the same renderer. No actual control,
consumer, narrow or UI PASS is claimed until its specified checks finish. The
F1 list and its pending physical-node binding correction are outside this scope.

Step2 adds ButtonConfig (typed size/layout/activity) and button_control, which
returns a styled Button with an input guard. Disabled or loading refuses a
supplied on_click callback; completion renders an enabled control again. The
same renderer replaces any leading glyph with LoadingSpinner Small20 and keeps
the label. New Outline uses the source borderAccent and transparent fill;
catalog Add chooses Small28/px12/font12-medium. Explicit static muted artwork
is replaced by the button's own variant ink while loading, as the source does.
A headless input regression deliberately supplies callbacks in every state.
Existing legacy label APIs remain unchanged; their disabled callback convention
is not retroactively changed by this new guarded entry. Source/narrow/UI proof
is still owed for this checkpoint.

Step3 unifies the existing outline compatibility wrapper with the same renderer.
All five variants now share the measured Small/Medium geometry, icon-only width,
text size/medium weight and pressed/disabled paint. Literal both-theme matrices
cover every variant/size/layout/content state. No area caller is edited.
The published outline compatibility wrappers retain direct text, no-shrink,
caller-selected tone and immediate-child bounds. They use the same frame/content
renderer through a private published-outline presentation. Typed entries retain
their wrapping labels. Existing caller files are listed below.

The signature-compatible button_with_kind and outline_button receive no Window
or App. Their Stateful<Div> is constructed before layout, so there is no reliable
current window layout at that boundary. They remain Pointer32/px16; no guessed
single-window global is introduced. Explicit typed callers get Touch44/px24.
This is the coordinator-authorized compatibility limitation, not phone parity
for every existing legacy caller. No other-owner caller migration is included.

Existing outline caller inventory (source scan at this change):
- `spikes/native/app/src/areas/about.rs`: 3 call(s).
- `spikes/native/app/src/areas/about/marks.rs`: 1 call(s).
- `spikes/native/app/src/areas/settings/overview.rs`: 1 call(s).
- `spikes/native/app/src/areas/settings/sync/frame.rs`: 1 call(s).
- `spikes/native/app/src/areas/settings/general.rs`: 1 call(s).
- `spikes/native/app/src/areas/settings/devices/revoke.rs`: 1 call(s).
- `spikes/native/app/src/areas/settings/devices/invitation.rs`: 2 call(s).
- `spikes/native/app/src/areas/settings/devices/rows.rs`: 2 call(s).
- `spikes/native/app/src/areas/settings/devices/row/tests.rs`: 1 call(s).
- `spikes/native/app/src/areas/settings/agents/detail/defaults.rs`: 2 call(s).
- `spikes/native/app/src/areas/settings/agents/detail.rs`: 1 call(s).
- `spikes/native/app/src/areas/settings/diagnostics.rs`: 3 call(s).
- `spikes/native/app/src/areas/settings/agents/detail/facts.rs`: 1 call(s).
- `spikes/native/app/src/areas/settings/agents/detail/sign_in.rs`: 1 call(s).

## Package-base4 integration checkpoint

The A8 merge of3cd0d3fc preserves its new outline_button_with_glyph signature,
its explicit glyph/text tone, and its exact actual-layout regression. A private
PublishedOutline presentation in the one frame/content renderer preserves the
published direct-label bounds and outline typography, rather than silently
changing the lead's Remove-machine action. The public typed APIs remain the
ordered content/loading/sizing extension; no area call is migrated. The canonical
incoming palette (including GREEN_500) is retained byte for byte. Both language
refresh hooks and all incoming kit exports are kept.

The earlier isolated6036 producer narrow run schedules-narrow-20261115T081346Z-b0948720
failed: native-fonts0, native-cargo101; no tests ran. It found one nonexistent
popup touch-target alias assertion, two redundant must_use attributes, an
unexhaustive button border match and a documentation lint. The original result
is retained as old-source failure. The base4 checkpoint corrects these; a new
minimal base4 producer must pass narrow checks before consumer handoff. No whole
F1 gate or UI pass is claimed here.

## Multiline source check

Paseo buttonSm/Md and text styles add no vertical label padding. New typed
content therefore wraps with only its line height and the button's border;
the historical filled-label APIs explicitly retain their existing four-point
label inset through a private compatibility presentation. Published outline
entries retain direct text. This keeps existing callers unchanged while source
content (including catalog Add) avoids eight invented points on a multiline
label. A real-layout regression compares two-line Medium42/Small34 with legacy
filled50, in both themes; narrow execution is still owed for this correction.

Exactef93023a Clippy passed; Kit tests ran134/1/0. The new layout fixture's
120-point width caused its longer second line to wrap again in the test font,
so it measured62 instead of the intended two-line42. The correction uses A\nB
to guarantee the fixture precondition. Production code and the literal height
assertions42/34/50 are unchanged. This is a fixture repair, not a changed expected
result or an assertion that actual long consumer labels have passed UI review.

## Corrected producer narrow result

Isolated code8258410623994773e75dd061fa282c7e09ade8ba passed Kit all-target
Clippy (schedules-narrow-20261115T090704Z-82584106) and Kit135/0/0 tests
(schedules-narrow-20261115T090742Z-82584106). Both runs executed native-fonts
and native-cargo with0/no skip; actual Kit Checking/Compiling is in the logs.
Steps SHA256: Clippy3c0334b5bbe1aea6a3c7c34a9bbee62341908c536e3b5815ff428090463ba248,
testsa0e91058807848ce7fb7bf28ee6bf83c48ed2fdea6cb9c287d9489037511e0e5.
Immutable13-file Rust formatting checks are byte-identical. Earlier failed
runs remain distinct; no UI/OS/S2/G3 first-use or F1/full-primary pass follows.
V1 source review is requested next; consumers wait for that bounded disposition.
The primary app tree merged074ade16; that merge changed only documentation
because the complete approved G4 prerequisites were already adopted.

V1 independently accepted isolated82584106 SOURCE and exact Kit narrow scope
(135/0/0), including all16 manifest blobs and unchanged published outline test
body. S2/G3 received that exact code plus result30d9c165, with the typed API and
newer-glyph union constraint. This is a shared producer handoff, not consuming
UI/OS/accessibility or full F1 approval; legacy disabled/domain conventions and
final feature-package evidence remain explicit. No unchanged gate was rerun.
