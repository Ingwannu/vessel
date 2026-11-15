//! Cron fixtures from Paseo's schedule tests, used through the node's
//! real preview call and by its calculation tests. DST differences stay
//! in Vessel's ADR 0033 fixtures rather than changing these expected times.

/// Expression, zone, cursor, and expected next UTC instant.
pub const CRON_CASES: &[(&str, &str, &str, &str)] = &[
    (
        "15 9 * * 1-5",
        "UTC",
        "2026-01-05T09:14:30Z",
        "2026-01-05T09:15:00Z",
    ),
    (
        "0 9 1 * 1",
        "UTC",
        "2026-10-17T00:00:00Z",
        "2026-10-19T09:00:00Z",
    ),
    (
        "0 9 1 * 1",
        "UTC",
        "2026-10-30T10:00:00Z",
        "2026-11-01T09:00:00Z",
    ),
    (
        "0 9 1 * *",
        "UTC",
        "2026-10-17T00:00:00Z",
        "2026-11-01T09:00:00Z",
    ),
    (
        "0 9 * * 1",
        "UTC",
        "2026-10-17T00:00:00Z",
        "2026-10-19T09:00:00Z",
    ),
    (
        "0 9 */2 * 1",
        "UTC",
        "2026-10-19T09:00:00Z",
        "2026-11-09T09:00:00Z",
    ),
    (
        "0 9 1-31 * 1",
        "UTC",
        "2026-10-17T00:00:00Z",
        "2026-10-17T09:00:00Z",
    ),
    (
        "0 9 1,15 * 5",
        "UTC",
        "2026-10-17T00:00:00Z",
        "2026-10-23T09:00:00Z",
    ),
    (
        "0 9 1 * 1",
        "America/New_York",
        "2026-10-17T00:00:00Z",
        "2026-10-19T13:00:00Z",
    ),
    (
        "0 9 * * 1-5",
        "America/New_York",
        "2026-01-05T13:59:30Z",
        "2026-01-05T14:00:00Z",
    ),
    (
        "0 9 * * 1-5",
        "America/New_York",
        "2026-07-06T12:59:30Z",
        "2026-07-06T13:00:00Z",
    ),
];
