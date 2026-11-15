# Settings integration compatibility

[Decision Log]
- Purpose: keep the reviewed shared header and machine-book tests buildable
  together with Agents after merging the desktop base.
- Constraints: the touch target has one canonical definition, and a test's
  explicit State constructor must initialize the same hubs as production.
- Alternatives: restore a removed popup constant; omit the existing Agents
  hub from the machine test state. Neither matches the integrated API.
- Choice: the two header references use `breakpoint::TOUCH_TARGET`, still
  44 points; the machine actor fixture initializes `agents::Hub::default()`.
- Why: both compile blockers arose at the same integration boundary. Either
  correction alone leaves that merged baseline uncompilable, so they travel
  together as one compatibility correction without drawing changes.
- Cost: the explicit fixture continues to require an update when a required
  State field is added, rather than hiding missing fields behind defaults.

The header's reference remains Paseo's `components/headers/screen-header.tsx`
and `compact-header.tsx`. No size, colour, callback or animation changes.
Its first-consumer visual review remains separate from source compatibility.

Validation on the integrated native workspace: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, and `cargo test` passed;
491 tests plus two documentation tests. The same frozen tree contains
pending Agents drawing work; that work is not delivered or visually
approved by this compatibility correction. The main workspace was not
changed by these fixes, and no additional main-workspace run is claimed.
