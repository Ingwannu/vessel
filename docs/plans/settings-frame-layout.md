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

## Checked receiving source before the final gate

Executable a80e564a2d64c9ba8852d5b497177932ebe1317b:
- Strict settings2-narrow-20261116T012323Z-a80e564a: fonts0 and View/App
  all-target Clippy0. This run actually checked Kit/App; unchanged View had
  already compiled in the earlier source run, which is not relabelled fresh.
- Runtime settings2-narrow-20261116T013345Z-a80e564a: fonts0, cargo0;
  actual App binary selection ran all five new real-root cases,5/0/0 in1.20s,
  after468s cold build. All four native crates compiled in that runtime run.
  No whole-gate or visual PASS is inferred.

Original raw logs/specs/steps are retained in /tmp/work/m12-settings4/
frame-strict-a80 and frame-tests-a80, with per-file full SHA receipts alongside.
frame-source-audit.json verifies the moved section composition matches base13
except the declared form argument and equal spacing-token substitutions.
frame-union-scope.json records the empty root/protocol/web delta. Earlier strict
failures remain retained; none is classified as infrastructure or substituted.

The next single final native gate/export will pin this source plus these actual
result/docs corrections. Fresh captures must use that verified exported binary,
private fake fleet and authenticated display under UID/GID1000. Existing captures
from selection/rename do not count as new frame proof. Hardware/physical IME,
OS accessibility, full E1/F5 and shared rename sibling occlusion remain outside
this bounded change. No service, account or product permission changes occurred.

## Quota stop checkpoint (2026-11-16 03:42 UTC)

Final checked source927680a90ba2052fb83fb0c6e85c937ac1f2c4b0 ran six native steps
rc0 in settings2-20261116T014900Z-927680a9: fonts/fmt/quality/Clippy/build/test.
Raw test totals are1618/0/2, including the five new General/frame cases. Only
capture-export failed: the old immutable submission pinned Root23 below the
new app-capture minimum25. The whole run's overall rc1 is retained; this is not
a Rust failure or a successful export. Native result receipt is retained locally.

T52f83380 and lead2c964ed8 authorized one same-source build/export-only recovery:
settings2-20261116T031322Z-927680a9, --steps native-build --export-app
--shared-root-fleet base12, /tmp/work/m12-settings4/rg-frame-export-927.
It is the SOLE queued E2 VM job. No unchanged whole or Root build is repeated.
Recovery uses approved Rootb5b46357/protocol25, separately identified from AppRooted4.
This docs-only WIP checkpoint does not change the pinned executable source.

Export recovery, every new frame capture and final V4 review remain UNVERIFIED.
No final review row has been submitted: wait for actual complete capture evidence.
G3 explicitly confirmed no checked private-picker resize hook exists (31c08d3f);
that picker remains separately UNVERIFIED, not an observed failure or part of
General5. No back/load workaround, private-state access or new owner API was used.
No own App/fake fleet/Xvfb was started. Two isolated capture-helper cleanup probes
were reaped. No production account/config/credential/service permissions changed.
