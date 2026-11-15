# F1 shared host selector

[Decision Log]
- Purpose: reproduce HostFilter through the one existing select/combobox engine.
- Existing assets: select::trigger draws SettingsSelect's rounded outline;
  combobox takes an exact width, already owns sheets, rows, input and motion.
- Alternatives: a schedule-local pill/popup, a forced fixed-width list, or
  changing the existing callers' defaults. All would duplicate or regress a
  shared surface.
- Choice: typed named select presentation/content plus typed fixed/content
  combobox sizing; old entries delegate unchanged. The new body/placement child
  keeps picker.rs below its measured shrinking record. No second renderer.
- Why and cost: source-specific geometry and natural label measurement without
  string or host logic in Kit. The screen still owns visible choices/callbacks.

Pinned reference: components/hosts/host-filter.tsx1–144 (whole), host-picker.tsx
HostStatusDotSlot and HostPicker default options; components/ui/combobox-frame-
style.ts (whole). HostFilter opts out of search and opens bottom-start. Its
trigger uses gap6, px12/py6, radius6, border1, medium14, leading14. Colours are
surface1 / hover2 / pressed3 with no open wash. Existing Settings disabled
opacity0.5 must win over open0.85; the regression asserts that combination.

Content width: measured anchor minimum, max(400, minimum), fallback200 only
without an anchor. Fixed mode and the old entry remain exactly fixed. The
compact sheet remains full width. Geometry tests include a small trigger, a
normal trigger, an anchor above400 and no measurement, along with both-theme
literal select colours. Actual intrinsic layout, long names, keyboard/pointer
selection, clipping and sheet first use still belong to the final F1 evidence.

The coordinator explicitly permits dot6 in the14-point slot until E2 supplies
a reviewed dot8 seam; no dot vector/control is duplicated. This is the only
intentional temporary leading-size difference. The HostErrors radius issue is
separate and awaits G4's corrected producer. These source changes do not settle
menu focus HOLD or claim a product UI pass. Narrow Kit results follow on the
immutable commit; no extra whole gate is run for this shared dependency.

The extracted sheet uses the already existing popup::sheet_scrim token, whose
paint is exactly the old rgba value; the obsolete picker hex allowance is
removed rather than moved or enlarged. The picker record shrinks829→672.
