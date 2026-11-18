// The words a diagnostic's text uses for a wire value (ADR 0111): the same
// rule as `vessel_proto::diagnostics::text::words`, so a report copied from
// the page reads as one from the app or the CLI.
//
// [Decision Log]
// - Purpose: one text for a report on every surface, without a table of
//   words per value kept in two languages.
// - Rule: `null` is "none", a boolean "yes" or "no", a string its text with
//   `_` read as a space, a list its items joined by ", ", and an object its
//   tag (`state`, `reason` or `kind`) then its other fields as
//   "name: words", sorted by name, in parentheses (or alone without a tag).
// - Held to the Rust renderer by the fixtures in `src/wire/fixtures/`.

const TAGS = ["state", "reason", "kind"] as const;

function spaced(text: string): string {
  return text.replaceAll("_", " ");
}

/** `value` (a wire value as JSON gives it) in words. */
export function words(value: unknown): string {
  if (value === null || value === undefined) {
    return "none";
  }
  if (typeof value === "boolean") {
    return value ? "yes" : "no";
  }
  if (typeof value === "number") {
    return String(value);
  }
  if (typeof value === "string") {
    return spaced(value);
  }
  if (Array.isArray(value)) {
    return value.map((item: unknown) => words(item)).join(", ");
  }
  if (typeof value === "object") {
    const entries: [string, unknown][] = Object.entries(value);
    const record = new Map(entries);
    let tagKey: string | undefined;
    let tag = "";
    for (const key of TAGS) {
      const candidate = record.get(key);
      if (typeof candidate === "string") {
        tagKey = key;
        tag = spaced(candidate);
        break;
      }
    }
    const fields = entries
      .filter(([key]) => key !== tagKey)
      .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0))
      .map(([key, field]) => `${spaced(key)}: ${words(field)}`);
    if (tagKey === undefined) {
      return fields.join(", ");
    }
    return fields.length === 0 ? tag : `${tag} (${fields.join(", ")})`;
  }
  return "unreadable";
}
