# Effective Settings form from the shared sidebar decision

[Decision Log]
- Purpose: keep Settings sections reachable when the nav and its720-point
  content minimum cannot share the row, including800 where global PHONE
  is already false, and a hidden desktop sidebar at any width.
- Existing assets: the accepted S2 panels::sidebar_layout/Sharing result,
  Shell's existing pushed/section state, nav::compact, PageLayout and G3's
  DetailHeader callback through one kit screen_header renderer.
- Choice: derive one typed Settings form from that result. A fitting dock
  uses split navigation; Sheet/Hidden/TooNarrow use the existing list/back
  flow. Remove both temporary Settings exceptions atomically: the sidebar
  PHONE guard and the pure policy's TooNarrow dock allowance.
- Why: an independent width check or hidden nav without a back/list frame
  can strand navigation or duplicate it at600-767. A separate control
  renderer would split shared geometry and input ownership.
- Constraints: no new width/breakpoint constant. Keep global PHONE and body
  density unchanged. Existing compact list/header controls provide the
  stacked form; existing section callbacks/state choose destinations.
  Keep S2's ordinary admission0/width400 and independent Explorer targeting.
- Costs: desktop windows with hidden/insufficient Settings nav use list/back
  even though their form controls retain desktop density. This is explicit,
  rather than pretending every stacked form is a physical phone.

Source begins at announced base13 2ebeedf9 in a clean App branch, not the
older mixed Settings4 branch. It contains the accepted selection package
and corrected S2 policy. K's portable notifications-only92ad is NOT imported
or assumed checked here; Terminal live/ordered-effect producers remain separate.
S2's PreparedRefusal body/seam edits are outside this frame scope.

Existing real-root GPUI setup will be shared by visibility only, without
another Shell/input engine or fake transport. Meaningful receiving cases
cover narrow list -> section -> back, explicit open-to-section, resizing
across sharing boundaries, hidden/wide preference, one nav/header and
retained section/body density. Captures and a final package gate/review
remain owed; this plan does not mark source or runtime complete.

## Source checkpoint

The frame now derives `Form::{Split,Stacked}` from the existing sidebar
arrangement and removes both temporary Settings exceptions in the same change.
The section-page composition moved into `settings/form.rs` by responsibility;
its body still receives the original global phone density. A form transition
closes the existing combobox anchor without applying a choice. The existing
real-root fixture is shared only through test visibility; four new cases cover
actual navigation, pointer back/list flow, resize/density and popup retirement.
Debug selectors identify existing surfaces and actions, with no release paint.

Local source checks: native size709 files/13 shrinking exceptions, token audit
none new (one obsolete settings/mod.rs px-step record removed), diff whitespace0.
These checks do not compile. Strict checks, all four new runtime cases, final
native gate/export, inspected captures and receiving review remain unverified.
No file/account permission or service changes were made.

## First strict result and popup completion

Exact933fe96e remote settings2-narrow-20261116T011543Z-933fe96e compiled
View/Kit/Link/App but failed strict101 on four authored test closure statements
missing final semicolons. The raw result remains in frame-strict-933; no runtime
or strict PASS is inferred. Those four punctuation fixes leave expectations and
product behavior unchanged. A separate source audit also found that the existing
Language popup needs the same form-transition retirement as the shared choices.
The controller now invokes its existing close method, and one new real-root
click/resize case verifies the actual picker closes without changing language.
No new language state, choice, callback or word is introduced. Five receiving
runtime cases, corrected strict, final whole/export/captures/review remain owed.

The corrected c722 run settings2-narrow-20261116T012159Z-c722ab0d
stopped at compile101: the new Language debug selector required the existing
InteractiveElement trait import. This source correction adds only that import;
raw logs remain in frame-strict-c722. It does not supply runtime proof.
