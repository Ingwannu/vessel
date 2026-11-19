// A project's lettered identity square where it has no icon of its own
// (TRACKER 809): Paseo's `ProjectIconView` fallback, its ten colours
// (styles/identity-colors.ts), its key hash and its initial
// (utils/project-display-name.ts).
//
// [Decision Log]
// - Purpose: a project without an icon is told apart by colour and
//   letter, as in Paseo, instead of one folder mark for all.
// - Constraint: the native app draws the same square (its
//   `project_icons.rs`), so the colour must come from what both know.
// - Choice: Paseo's table, order and hash, keyed by the project's id
//   (Paseo keys by its project key or [server, project]); the page and the
//   native app then colour a project alike.
// - Cost: a project removed and added again gets a new id, so possibly a
//   new colour.

/** Paseo's ten identity fills, in its order: the hash indexes this list,
 * so reordering recolours every project. Muted tones in one contrast band
 * against the white letter (Paseo's note: colour identifies, it does not
 * rank). */
const IDENTITY_FILLS = [
  "#7a6aa8",
  "#3d7ea6",
  "#388068",
  "#a4673a",
  "#b05c80",
  "#6a70b8",
  "#368080",
  "#b06260",
  "#8f7838",
  "#5179b0",
] as const;

/** Paseo's `hashIdentityKey`: 31 times, over each character's first UTF-16
 * unit, kept to 32 bits. */
function hashIdentityKey(key: string): number {
  let hash = 0;
  for (const character of key) {
    hash = (Math.imul(hash, 31) + character.charCodeAt(0)) >>> 0;
  }
  return hash;
}

/** The fill of the square for `key` (a project's id). */
export function identityFill(key: string): string {
  return (
    IDENTITY_FILLS[hashIdentityKey(key) % IDENTITY_FILLS.length] ?? "#7a6aa8"
  );
}

/** Paseo's placeholder initial: the display name's last `/` segment's
 * first character, upper-cased; empty for an empty name. */
export function projectInitial(name: string): string {
  const trimmed = name.trim();
  if (trimmed === "") return "";
  const segments = trimmed.split("/").filter((segment) => segment !== "");
  const last = segments.at(-1) ?? trimmed;
  return last.charAt(0).toUpperCase();
}
