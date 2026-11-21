// The wire's times as numbers, for the pull request pane's rules. The
// native rules (view/src/workspace/pull/) hold chrono's `DateTime<Utc>`;
// the page's wire types (src/wire/proto.ts) carry the RFC 3339 text serde
// writes for it.
//
// [Decision Log]
// - Purpose: one place where a time from the node becomes milliseconds
//   since the epoch, so every rule compares numbers and a test passes its
//   own `now`.
// - Constraint: `Date.parse` keeps milliseconds, chrono nanoseconds. A
//   rule that truncates to whole seconds can differ by one only when two
//   times sit less than a millisecond either side of a second's boundary.
// - Choice: a text that does not parse reads as absent (`null`), never as
//   the epoch or as NaN, so a broken time hides a timing instead of
//   drawing "56y ago".

/** Milliseconds since the epoch of `text`; null without a text, or when
 * it does not parse. */
export function instant(text: string | null | undefined): number | null {
  if (text === undefined || text === null) {
    return null;
  }
  const at = Date.parse(text);
  return Number.isNaN(at) ? null : at;
}
