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
