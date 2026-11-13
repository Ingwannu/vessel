# Native Settings: Agents and Models

Track G3, branch `backend/m12-app-agents`. The contract is milestone
11's track G and ADR 0032. This track changes no node or wire type.

## Existing assets and choice

Modify and reuse: `view::agent_status` and `view::models` already port
the page's discovery, option and model-row fixtures. Settings' frame,
machine chooser, cards and select remain the one implementation.
`link::Prefs::choices` already supplies the next conversation's defaults,
including added agents; no second preference store is introduced.
Standard Rust and the existing client/protocol types suffice.

## Slice 1: facts before drawing

`agent_status::readiness` extends discovery with the facts from
`agent.status`: installation and conversation support, credentials and
unknown reasons, version, and the relevant vendor installer part.
All sentences live in `view::text::agents` in English and Korean.
The existing page tests (`data/agent-status.test.ts`, `agents.test.ts`)
stay intact. New cases cover every readiness variant and failure step.

Differences from Paseo and GOAT: no enable switch, because the node has
no enable operation; no in-app sign-in, because ADR 0032 exposes help
only. A completed handshake is never presented as authentication.

No screen changes in this slice; captures do not apply. Live agents,
real registry fetches and installers are not exercised. Screen and
fake-fleet verification follow in the next slices.

Problems met: the first native Clippy pass rejected a missing `Debug`
implementation on the words table and a redundant version accessor
closure; both were fixed, without suppressions. An initial gate command
was run from the root workspace by mistake; its formatter changed no
root source, and its Clippy process was stopped by its recorded process
ids. It is not counted as a native gate result. The native workspace's
first test run builds the pinned GPUI tree from a cold target directory.

Gate: native `cargo fmt --check`, `cargo clippy --all-targets -- -D
warnings` and `cargo test` passed, whole workspace, 308 tests passed and
none failed. No main-workspace source changed; its gate is not claimed.

## Slice 1 review correction

The first slice was not accepted in review. This correction keeps its
separate installation and sign-in facts, and adds:

- A typed short badge after Paseo's provider rows: Available, Not
  installed, Needs attention, Error, Not known. Installation/chat support
  alone determines this badge; it never claims authentication. Sign-in
  remains a separate sentence. There is no disabled state without an
  enable operation.
- Complete, plain-language instructions. An older agent requires an
  agent update and another check; a newer one requires a Vessel update.
  Protocol version numbers and connection jargon do not stand in for an
  instruction. Check instructions interpolate the actual `check_status`
  action label; adapter instructions interpolate `install_adapter` and
  appear only where the node offers that action.
- A labelled machine message separate from the translated sentence.
  An English diagnostic is retained verbatim, rather than becoming the
  end of a Korean sentence.
- Typed sign-in steps for the profile's account-selector variable, the
  exact node-provided command, and each agent-provided method. Without a
  command or method the step explicitly uses the agent's own interface.
  Nothing guesses a login command, runs it locally, or exposes a variable
  value. The screen will draw command steps with the kit's copy control.
- Whole-sentence English/Korean expectations for every installation,
  sign-in and unknown-reason case, every badge/variant and all sign-in
  step shapes. Prefix/substring assertions were removed.

The shared badge landed in `840a941` and is cherry-picked here. This
correction imports its `view::badge::Variant` directly; the temporary
local variant module was deleted in the same change as the import swap.
The shared app mapping and kit control are reused when the screen lands.
No screens or external calls are part of this correction.

Correction gate, after shared-badge integration: native `cargo fmt
--check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all
passed in order, whole workspace: 315 passed, none failed. Captures do
not apply to this view-only correction. The first review pass also
caught a test table over the function-size limit and missing code marks
in documentation; the table became a fixture helper and documentation
was corrected, without lint suppressions.

## First screen: request and output rules

The lead's app frame was merged before this slice; its whole native gate
passed (316 tests). The screen reuses the settings machine chooser,
section/card/row, shared badge and the new-chat option controls. Defaults
use the existing per-machine/per-agent `Prefs::keep_choices` store.

[Decision Log]
- Purpose: a machine switch must not cancel or misattribute an install.
- Constraints: node calls are asynchronous; output is paged and a process
  may have exited before all its retained pages have been read.
- Alternatives: one visible-machine snapshot, unbounded histories, or
  request logic inside callbacks.
- Choice: `view::agent_status::pages` keeps eight machine pages, typed
  book keys and checked request tickets. Active pages cannot be evicted.
  The link bounds concurrent work to four tasks, waits for output with
  channel backpressure, validates byte cursors, and drains terminal pages
  until an empty terminal page before reporting completion.
- Why: a later failure message must not disappear merely because the
  first page already said the installer exited. A lost output connection
  is not an installer failure. Stale responses cannot complete a new job.
- Costs: each page retains at most 1 MiB; the node has no durable install
  history. A completed job invalidates its machine snapshot so returning
  to that machine reads the changed installation facts.

Missing-program instructions name the actual Install control only where
the node offers it. Korean button references put a noun after the quoted
label, so translating a label cannot break its grammatical particle.
Commands, paths and individual argv values use fixed width and the kit's
copy button with a 44-pixel target. This local `command_box` composition
is potentially shareable; it is not a second kit control. Added program/arguments are quoted for sh, bash or zsh only when the
node’s OS is known to use Unix command syntax. Every value is single-quoted,
including empty values, with embedded apostrophes escaped. The label says
which shells and explicitly excludes account/environment configuration.
Other platforms retain individually labelled argv values: the node names
no shell, and inventing cmd.exe or PowerShell syntax could change what runs.
The CLI’s display-only space join was not reused for an executable copy.

### Verified limits to the fleet

The node's status result for a failed ACP check is an installed program
with unknown sign-in and a failed-check reason. The screen shows that
fact; it does not manufacture an installation-error badge.

The pinned ACP 5.0 library prevents exercising the node's explicit
older/newer-version states with its normal fake. Its responder's
`jsonrpc/protocol_compat.rs::outgoing_response_to` overwrites the returned
version with the negotiated one. Its client's `incoming_initialize_response`
rejects an unsupported or different version before `vessel-agent` can
reach its explicit version comparison. An attempted fake override was
therefore removed rather than claiming a successful fixture. Both states
remain covered by exact bilingual view tests; runtime captures of those
two states are unavailable without a node/dependency change, outside this
track's scope.

The registry address cannot be set on the node's CLI or in its environment,
so a fixture registry cannot be served to this fleet. Registry rules will
use the node catalogue test's fixture shape; registry screen captures will
be labelled as the live public list. No actual package installation is
allowed in these checks.

The list opens a detail view in this same Settings section, after Paseo’s
provider-row navigation, with a return-to-list button. This avoids
placing details below a potentially long agent list. Older nodes show the
update instruction instead of administration rows that name unavailable
controls. The current native frame still labels this nested view Agents;
the agent’s own name is its detail-section heading.

The accepted shared Alert is cherry-picked as `4faa028`. Merging it after
the lead required one conflict resolution: the glyph list has 43 entries,
preserving the lead’s 41 and the two added circle glyphs. Failed requests,
status-check results, installer outcomes and attributed machine messages
use this one kit control. Their notice variants come from pure view
rules; a lost output reader warns without claiming a failed installer.

The first visual pass caught full-width action pills and a list sentence
pointing to an Install button hidden in details. Action buttons now fit
their labels, keeping the kit’s 44-pixel target. The list uses a separate
whole-sentence summary of the same typed readiness; details retain the
accepted complete instruction directly above the control it names. Both
places have exact bilingual fixture expectations.

Each list row also states sign-in separately beneath the installation
badge. In particular, a discovered chat adapter with a failed sign-in
check retains the installation badge the node supports, while explicitly
saying that the sign-in check failed. Unknown sign-in never becomes a
successful-looking installation-only row. Details retain the complete
reason and the node’s sign-in steps.

The default-choice popup uses the composer’s 360-pixel model-list width,
its grouping metadata, and the shared select’s end-aligned anchor. The
220-pixel generic preference width cut ordinary model names unnecessarily
in the first capture. The kit retains Paseo’s one-line, ellipsized option
labels/descriptions at the final viewport boundary; no second list is drawn.

A completed silent installer must not say it is still waiting for output.
The empty-log sentence is therefore neutral for both a running and a
terminal job: no output was received. Its exact English/Korean regression
expectations prevent the old waiting sentence from returning.

Callbacks retain the machine key that painted their control and discard
an event after the visible machine or section changed. A queued click
cannot redirect an install, a check or a defaults write to another machine.
Opening a row is idempotent; the return action clears selection explicitly.

The first navigation pass also exposed the shared outer scroller carrying
a detail’s offset into another machine or the agent list. A small frame
slot supplies a scroll element identity; Agents chooses a distinct identity
for each machine’s list and agent detail. The frame/card implementation
remains shared. This optional scroll-identity slot is shareable with other
nested Settings pages; its regression test distinguishes lists, details,
machines and book names containing punctuation.

The long-command interaction check caught a layout boundary: a no-wrap
text child could paint beyond its measured box without contributing its
whole width to the scroll extent, and the frame accepted the horizontal
wheel as vertical movement. The local command composition reuses the
kit’s font measurement for an unshrinking content width and stops further
scroll propagation after its child consumes the event. Clipboard byte
parity alone was not counted as proof of scroll access.

### Shared values and verification

The lead’s spacing commit (`5e3e78a`, cherry-picked as `a37b547`) is used
for every gap and inset in this area. Finger targets reuse the kit’s
`popup::row_height(true)` and the command radius uses its existing value;
these substitutions preserve the captured geometry.

The capture set covers English/Korean, dark/light, and 1280/390 widths.
Every reported image was opened and inspected. It includes available and
unknown status, a missing built-in with an install offer, a missing added
program without one, the absent adapter and its attributed machine
message, failed checks, signed-out profiles and both kinds of sign-in
help, long added commands, and fake installation progress and failure.
The model list was also checked with the fleet’s sixteen long model names
and multiline descriptions, including the bottom of its scrolling list.
Original exploratory captures of the broken horizontal scroll are excluded
from the reported folder.

Interaction checks: the copied 461-byte launch command parses back to the
fixture’s exact program and argv, using an independent shell parser and
without executing it. Model, effort and mode changes were saved through
the existing store and reset through the UI. A fake adapter install was
held running while switching to another machine, failed there with exit
7, and retained both output lines on return. Only the fake npm ran; it
neither downloaded a package nor changed any installation. Private empty
and unreachable-machine books exercise the empty and request-failure
screens without changing the working fleet.

The app and its fake processes ran under the same development UID/GID.
Scratch homes and node data were private (0700), and the kept choices file
was 0600 under the private QA launcher’s `umask 077`. The existing shared
Prefs writer recreates its file using the caller’s umask: a later default
launch produced 0664 inside the private 0700 directory. The QA launcher
and documented fake-run command now set 077, and a repeated UI save and
reset verified 0600. This shared persistence boundary was reported to the
settings owner; its production behavior was not changed by this area.
No production permissions changed. No real chat prompt or real
package installation is claimed.

A shared select boundary remains: keeping a phone sheet open while
resizing to desktop retains the trigger anchor saved at opening. Closing
and reopening uses the correct desktop anchor. The initial-width popups
were checked; this open-overlay resize transition is reported to the lead
and is not claimed as passing.

Final screen gate after the shared spacing integration: native
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test` passed in order, whole workspace: 342 tests passed, none
failed. The GPUI scroll regression checks the actual laid-out bounds,
not fixed event coordinates. The interrupted earlier run is not counted.
The fleet’s shape assertions passed, and both shell scripts pass
`bash -n`. No main-workspace source changed; no main-workspace gate is
claimed. Transient request loading was not held for a synthetic capture.

## Screen layout review and refreshed baseline

The coordinator rejected the layout of `16d5a75`; its request tickets,
retained output, exact copy and existing defaults store stay. The redraw
follows Paseo's `screens/settings/providers-section.tsx` (`ProviderRow`,
`StatusIndicator`), `components/settings/headings/settings-section.tsx`,
and `styles/settings.ts`. Its detail follows
`components/provider-settings-host.tsx` through
`components/provider-diagnostic-sheet.tsx`, with the native page's shared
header (`components/page-layout/index.tsx`, `components/headers/back-header.tsx`).
The rejected screen captures remain review evidence, not acceptance of
that layout.

The lead baseline was merged again at `b86f0c7`. Conflicts retain both
Agents and Devices registrations, request hubs and browser replies, both
README additions, and the lead's current shared Badge/Alert implementations
and complete 54-glyph inventory. The merge adds no node or wire changes;
two testkit source changes belong to the merged lead.

Merged-tree checks passed, detached: root `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test -p vessel-testkit` (14 passed), then the three whole native
checks (396 passed). The other root-workspace tests were not run. The
current CLI, node and fake binaries were rebuilt for the merged fleet.

### Shared section action slot

The approved `kit::settings::section_with_trailing` follows Paseo’s
`components/settings/headings/settings-section.tsx`: title and optional
trailing action in one centered head, 8 between, 12 above the card, 4
inside its left edge, and 24 below the section. Existing `section` calls
delegate with no action. A GPUI layout regression checks the empty slot’s
geometry, the action’s right edge/vertical alignment, and the card gap.
This control-only change introduces no area-specific action or words;
its first Agents use is in the redraw.

Section-slot gate: all three whole native checks passed, 397 tests
passed and none failed. No root source changes in this control slice.
The layout regression exercises the slot without an area screen; the
Agents redraw will supply its capture of the section action.

### Provider-row rules before drawing

`agent_status::row` now projects one typed row after Paseo’s `ProviderRow`
and `StatusIndicator`: available, missing, needs attention, failed,
unknown, signed out, or loading. Only explicit sign-out changes an
otherwise available row; every other authentication fact stays in details.
Missing installation has the reference row’s warning dot, independently
of the catalogue’s muted badge. A relevant request uses a neutral spinner.

A positive model count comes from the same model control as the composer;
an unread/empty model list is not reported as zero. Only Available can
carry that count. Compact indicators cannot carry label/count fields and
compact rows omit the error paragraph. Expanded rows can carry a short
error only for a failed installation/chat check; normal state summaries
and authentication explanations are not row paragraphs.

Whole-label bilingual tables cover installation states/reasons and both
marker densities. Other fixtures cover every authentication outcome,
positive/empty/unknown model counts, request identity, and machine-scoped
section headings. These rules are not yet wired into the rejected screen;
that is the next redraw. Native fmt, whole Clippy and all 404 tests passed.

### Shared settings furniture review correction

V1 returned the first section slot: title flex growth/minimum width were
not in Paseo’s titleRow, and comparing a delegating wrapper with the same
None call did not establish reference geometry. The extra title sizing
is removed; the titleRow follows the reference’s row alignment and gap.
Tests now assert fixed head/card/section bounds from the reference values
and the existing type steps, independently for both public entry points.
First-consumer visual approval remains pending; the rejected Agents
captures are not used as approval for this control.

The approved `row_with_content` slot follows `styles/settings.ts`’s row
frame. `row_at` supplies its existing words/control to that same frame;
custom provider contents therefore do not duplicate padding or borders.
Fixed GPUI bounds check pointer insets, the 12-point control gap, the
one-point hairline and compact wrapping for both ordinary and custom rows.

The fixed row fixture uses the kit's existing Body line height of 21
(`fonts::Type::metrics`), not an assumed 20. The first run exposed that
wrong fixture value; implementation geometry was unchanged. The corrected
reference bounds include half-point vertical centering for a 21-point
label next to a 28-point control, and the wrapped row's 93-point height.

Corrected control gate: native fmt, whole Clippy and whole tests passed,
406 passed and none failed. The failed fixture run is not counted as a
pass. First-use captures of the corrected section and custom row remain
part of the following Agents screen review.

### Provider error-detail palette token

[Decision Log]
- Purpose: ProviderRow's error paragraph uses Paseo's palette.red[300],
  distinct from its semantic danger dot.
- Constraint: `Status` already exposes the shared blue[300]/amber[500]
  palette accents; existing danger signals keep their accepted values.
- Alternatives: an app-local literal or substituting the danger signal.
- Choice: one shared `Status::red_300`, fixed to the reference's #fca5a5
  in both themes, with an exact-value regression.
- Why: the source (`providers-section.tsx` errorText and `styles/theme.ts`
  palette.red[300]) deliberately uses a separate colour in this place.
- Cost: future themes preserve this fixed palette entry. First-use screen
  captures follow with the Agents redraw; this token alone is no UI pass.

Token-only gate: native fmt, whole Clippy and all 407 tests passed;
no root source changes. Exact red[300] is tested for both themes.

### Shared initial-loading shapes

The initial provider loading composition needs Paseo's shared
`components/ui/skeleton.tsx` primitive. V1 reserved it for this track.
One mounted animation invokes a caller's composition with one typed
opacity, shared by all shapes. Only surface2 rectangles fade; card
furniture stays solid. The one-second rise/fall uses Animated.timing's
default inOut(ease), whose Bézier (0.42,0,1,1) is verified against the
installed reference's TimingAnimation/Easing. A bounded polynomial
inversion matches it at f32 drawing precision without another dependency.
GPUI owns mount state/frame scheduling and reduced-motion policy; no
background task survives the element.

The actual GPUI probe checks shared shape opacity at mount and at a
controlled phase, unchanged furniture opacity, reference shape bounds, no further
animation callbacks after unmount, and the initial appearance on remount.
Numerical fixtures compare both halves with the reference timing curve.
Corner steps are source values: small 2, medium 6 (`styles/theme.ts`).
The following Agents composition will supply first-use captures; these
constant/geometry tests alone are not visual approval.

The first runtime probe failed its assumed zero y-origin: the GPUI root
filled the 80-point test viewport and centered the shapes at y30/y34. The
probe now explicitly sets that 80-point frame and checks those independent
centered bounds. The shared shape/pulse implementation is unchanged.

A subsequent probe exposed another wrong test assumption: advancing the
executor clock does not advance GPUI's unsynced Animation wall Instant.
The probe now supplies deterministic phases at the GPUI easing boundary
through the same private animation/draw path, retaining real mount and
frame-scheduling behavior. The default animation definition's period,
repeat policy and full timing curve are checked independently against
reference values. It does not claim a real 500 ms elapsed-time test;
first-use runtime captures will check the visible animation.

Skeleton control gate: native fmt, whole Clippy and all 409 tests passed,
none failed. The earlier failed probes are not counted as a pass. The
combined tree includes the provisionally reused Ghost control; no root
source changes or root test run are claimed for this control-only slice.
