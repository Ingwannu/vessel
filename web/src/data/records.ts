// Records keyed by a handle (`Record<NodeKey, V>`), walked without losing
// the key's brand: `Object.entries` and `Object.keys` give every key back
// as a plain string. The keys were branded when the record was built (by
// a frame, or by this page from keys already branded), so nothing is
// minted here: `for…in` over a generic record keeps the key's type.

/** `record`'s own entries, each key as the type it was keyed by. */
export function entriesOf<K extends string, V>(
  record: Readonly<Record<K, V | undefined>>,
): [K, V][] {
  const entries: [K, V][] = [];
  for (const key in record) {
    const value = Object.hasOwn(record, key) ? record[key] : undefined;
    if (value !== undefined) {
      entries.push([key, value]);
    }
  }
  return entries;
}
