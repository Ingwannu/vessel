# Scoped constructor test helper cut

[Decision Log]
- Purpose: keep the existing full-source scoped constructor regression within the test function length limit.
- Existing assets and constraint: the common producer already owns one sequential GPUI scenario using one retained field, and its original-address regression. They must retain every assertion and expected token.
- Choice: extract only setup, first-submit and final-release helpers and add the missing unit-tail semicolon. Setup is72 physical body lines, first-submit41, release23 and the scenario85. No lint allowance, product code or alternate fixture is added.
- Why: this preserves the actual event sequence and field identity without importing the separate minimal constructor producer or another owner's source.
- Effects: all16 existing assertion invocations and expected tokens are unchanged. The existing original-address test remains intact.

The one Rust file is byte-identical to the helper correction in54df7da8b. Its parent test file was independently verified byte-identical to the common producer baseline before this cut. The cut adds no root, Cargo, protocol, control, lifecycle, attachment or runtime implementation change. Static checks and byte parity do not supply a new combined compilation or execution pass; that consumer still owes its own exact checks.
