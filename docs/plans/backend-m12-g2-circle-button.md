# Source circular button presentation

Decision: 수정 사용. The existing button body, SVG and loading/input guard are
reused; no voice_button or second renderer. W19 owns the overlay, its current
state/labels, Mic/MicOff2.5-stroke assets and the requested filled-square asset.

Pinned1c00b365 components/realtime-voice-overlay.tsx (whole142lines), especially
17–18 size,55–94 controls,121–141 paint:44 square/full radius both layouts,
icons20/stroke2.5, surface0/border/foreground for unmuted, fixed red600/red800/
white for muted and Stop. There is NO pressed/hover override. Switching disables
both at0.5; only Stop replaces its icon with LoadingSpinner Small20. Current
Glyph::Stop is an outline and is not silently substituted for the filled Square.

[Decision Log]
- Purpose: voice controls match their own source geometry/tones in the shared kit.
- Constraints: generic sizes/paint and compatibility signatures stay unchanged;
  regular geometry fields cannot become ignored options on a voice preset.
- Alternatives: area paint/scaling, semantic destructive, another renderer,
  or overloading regular config with irrelevant density/size/kind fields.
- Choice: ButtonDesign is Regular(ButtonConfig) or Circle(CircleConfig). The
  latter stores only tone/state/activity and always owns44/20/static paint.
  button_with_design enters the SAME body; button_control delegates as Regular.
- Why: two complete typed configuration families, one SVG/frame/handler guard.
- Costs: this is a new explicit entry for source circles; no old caller migration.

K confirmed no competing fixed-red token producer and authorized the minimal
RED_600/RED_800 additions to canonical theme/palette.rs. Palette, Tint, generic
mapping, GREEN_500 and all old values/constructors are preserved. Fixed voice
red is not neutral.destructive. No new dependency or raw area colour is needed.

API: button_with_design(id,ButtonContent::Icon(ButtonIcon::new(glyph)),
CircleConfig::new(CircleTone::Surface|Danger,state).into(),tokens).aria_label(words)
.on_click(handler). CircleConfig::loading is Stop's switching state; mute keeps
its icon and uses Disabled. Callers still guard current target/pending/authority.
Role/label forwarding is not keyboard binding, OS or accessibility approval.

Both-theme literal geometry/palette/phase/icon cases and actual disabled/pending
click refusal/re-enable use the common renderer. Their Stop fixture tests frame
only, not a claim about W19's exact final icon. Old regular/published-outline
regressions remain unchanged. Narrow/source review and final W19 first-use,
layout, switching, actual glyph/OS/accessibility evidence are still owed.

The exact isolated5f714 Clippy compiled Kit and found the producer parent
lacked theme::icon::LG (present in the primary source), plus three redundant
qualifications in the new tests. The corrected producer explicitly appends the
canonical LG20 token and removes those test-only qualifications. No geometry,
paint, guard, old test or expected value changes. The old outcome stays red;
corrected narrow/source and final first use are still owed.


## Exact corrected producer result

The corrected isolated producer2b080afd passed the requested remote Kit suite:
Clippy0 (schedules-20261115T104948Z-2b080afd), Kit137/0/0
(schedules-20261115T111545Z-2b080afd). Both native-fonts/native-cargo
steps returned0 without a skip; changed Kit was compiled. Both specs use
--locked, short lane, priority0. Source5f714 remains the historical red
Clippy result, not relabelled corrected proof; the earlier VM143 interruption
is retained separately. The old regular/outline test files are byte-identical
to reviewed825, and canonical palette differs only by the two fixed reds.

This is author-reported narrow evidence awaiting V1's one bounded source review.
No whole F1, actual voice consumer/native UI, OS, switching lifetime, final
glyph, accessibility or authority pass is claimed. Consume only the isolated
producer after review; preserve newer exports/palette/inventory on integration.
