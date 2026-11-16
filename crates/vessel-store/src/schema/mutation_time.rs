//! One nullable clock append: historical activity is Unknown, never backfilled.

/// Integration re-points only this next-schema expression in merge order.
pub(super) const MUTATION_TIME_SCHEMA: usize =
    super::migrations::BACKGROUND_SCHEMA.saturating_add(1);

/// No client/event/creation timestamp can invent historical mutation ordering.
pub(super) const SQL: &str = "ALTER TABLE sessions ADD COLUMN updated_at INTEGER;";

#[cfg(test)]
mod tests;
