// Where a Markdown file's link or image points, as Paseo's preview
// resolves it (`components/markdown/image-source.ts`, pinned 0.15.4): a
// relative target from the Markdown file's folder, an absolute one as it
// is; a web, `data:` or `blob:` address is left to the caller. The page
// then reads it through the node, which serves files under the workspace
// root only, so a target is kept as a path relative to that root, and one
// outside it is not a file the preview can show (plan difference 7).

import type { RelPath } from "../../wire/proto";

/** What a target is. */
export type MarkdownTarget =
  | { kind: "web"; url: string }
  | { kind: "data"; url: string }
  | { kind: "file"; path: RelPath }
  | { kind: "none" };

const NONE: MarkdownTarget = { kind: "none" };

function decoded(value: string): string {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

/** `parts` with `.` dropped and `..` taking the folder before it; `null`
 * when a `..` would leave the root. */
function normalized(parts: readonly string[]): string[] | null {
  const kept: string[] = [];
  for (const part of parts) {
    if (part === "" || part === ".") continue;
    if (part === "..") {
      if (kept.pop() === undefined) return null;
      continue;
    }
    kept.push(part);
  }
  return kept;
}

function segments(path: string): string[] {
  return path.replace(/\\/gu, "/").split("/");
}

/** The file `target` names, written in the Markdown file at `from`
 * (relative to the root, whose folder on the node is `folder` when
 * known). */
export function markdownTarget(
  target: string,
  from: RelPath,
  folder: string | null,
): MarkdownTarget {
  const source = target.trim();
  if (source === "" || source.startsWith("#")) return NONE;
  if (/^https?:/iu.test(source)) return { kind: "web", url: source };
  if (/^data:image\//iu.test(source)) return { kind: "data", url: source };
  if (/^[a-z][a-z\d+.-]+:/iu.test(source) && !/^file:/iu.test(source)) {
    // `blob:` names memory of the page that wrote it; any other scheme
    // is not a file.
    return NONE;
  }
  const bare = source.replace(/[?#].*$/u, "");
  const path = /^file:\/\//iu.test(bare)
    ? decoded(bare.replace(/^file:\/\/[^/]*/iu, ""))
    : decoded(bare);
  if (path.startsWith("~")) return NONE;
  if (path.startsWith("/") || /^[a-z]:[\\/]/iu.test(path)) {
    if (folder === null) return NONE;
    const base = normalized(segments(folder));
    const full = normalized(segments(path));
    if (base === null || full === null || full.length <= base.length) {
      return NONE;
    }
    const inside = base.every((part, index) => full[index] === part);
    return inside
      ? { kind: "file", path: full.slice(base.length).join("/") }
      : NONE;
  }
  const directory = segments(from).slice(0, -1);
  const joined = normalized([...directory, ...segments(path)]);
  return joined === null || joined.length === 0
    ? NONE
    : { kind: "file", path: joined.join("/") };
}
