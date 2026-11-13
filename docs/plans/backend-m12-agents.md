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
