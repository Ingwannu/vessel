//! Synthetic response shapes from the vendors' documented/reference formats.
//! Shared by harness decoders, real-node tests and future fake-fleet consumers;
//! no network request or real login discovery belongs in this fixture module.
/// Claude's general and model-scoped usage, including reported zero.
pub const CLAUDE_USAGE: &[u8] = include_bytes!("../tests/fixtures/limits/claude-usage.json");
/// `ChatGPT` limits, credits and independent code-review allowance.
pub const CODEX_USAGE: &[u8] = include_bytes!("../tests/fixtures/limits/codex-usage.json");
/// A weekly window appearing only in the secondary slot.
pub const CODEX_WEEKLY_ONLY: &[u8] =
    include_bytes!("../tests/fixtures/limits/codex-weekly-only.json");
/// An unreported period and separate feature allowance.
pub const CODEX_UNKNOWN_PERIOD: &[u8] =
    include_bytes!("../tests/fixtures/limits/codex-unknown-period.json");
