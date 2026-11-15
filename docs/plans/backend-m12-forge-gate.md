# Explicit coverage for the forge fixture feature

The root workspace gate and CI skip required-feature binaries. ADR 0060's
separate vessel-forge-fixture and feature-only launcher tests therefore need
explicit Clippy/test steps. This follow-up changes the gate, not the product
binary, wire contract or fixture behavior.

[Decision Log]
- Purpose: a green default workspace must not hide a broken fixture executable.
- Existing assets: keep the current CI matrix, prebuilds and separate TS/web
  checks; reuse the existing fast source checks and Node runtime.
- Choice: a serial local scripts/gate.mjs runner plus the two explicit CI
  commands. The runner preserves its calling environment and stops on failure.
- Why: the feature remains test-only and disabled in normal product/release
  builds; making it a default feature would defeat that boundary. A separate
  runner makes the full local command sequence reviewable without extra tools.
- Limits: Linux execution is checked locally; hosted Windows/macOS CI and
  release artifacts are not claimed verified. No new external dependency.

Base candidate: 4660f784, on the owner's a365d295 correction. The pending
large merge on the original N3 worktree is kept separate; this small follow-up
uses its own worktree/branch so no native ancestry enters the node line.
The actual public runner passed on Linux through one shared slot, using
SSD test storage and the same environment for both A5 prebuilds and tests.
Format, size (769 files, 31 listed), boundaries (22 crates, 10 owned external
crates), default workspace Clippy and fixture-feature all-target Clippy passed.
Whole workspace: 113 suites, 1,973 passed, zero failed, 12 existing ignores.
Explicit fixture tests: four passed, zero failed/ignored (1.67 s). No lint,
assertion, timeout or ignore was weakened or added.

Evidence is in the track scratch's resumed directory: forge-gate.log and
forge-gate-summary.json; forge-gate-input-check.json pins 1,116 Rust/Cargo
inputs unchanged, and forge-gate-code-check.json checks the runner/CI/gate
policy/README inputs. These are selected source pins, not an attestation of
all ignored config, toolchain or environment state. Front/end one-minute load
14.05/29.71; available memory18,144/13,101MiB. The dedicated SSD scratch is
empty, UID/GID1000,0700, and no owned process remains. No live service or
production permission changed. Hosted CI was not dispatched; Windows/macOS
execution remains unverified. No new dependency or product Rust change.
No new pull-request listing call is added; row 5 remains separate. New calls
will use A6's shared unassigned-protocol accessor after it is merged.
