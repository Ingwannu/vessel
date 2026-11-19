// Add project's directory rows: the machine's recent project roots that
// match the field, then the machine's `folder.suggest` answer, and a
// typed path first. Paseo's `project-picker-options.ts`,
// `working-directory-suggestions.ts` and `shorten-path.ts` (pinned 0.15.4).
//
// Vessel's recent roots are the machine's projects' roots from
// `project.list` (plan difference 4); a machine below `folder.suggest`'s
// protocol gives no answer, so its rows are those and a typed path
// (difference 3).

import { scorePathMatch } from "../../text-match";

/** A directory row: a typed path ("Open this path") or a suggestion. */
export interface DirectoryOption {
  kind: "path" | "suggestion";
  path: string;
}

/** The home folder as `~`, for a macOS or Linux home (Paseo's rule). */
export function shortenPath(path: string): string {
  return path.replace(/^\/(?:Users|home)\/[^/]+/u, "~");
}

/** Whether typed text is a path the node can open: absolute, `~`, a
 * Windows drive, or a UNC share. */
export function isOpenableProjectPath(query: string): boolean {
  const trimmed = query.trim();
  return (
    trimmed.startsWith("/") ||
    trimmed.startsWith("~") ||
    trimmed.startsWith("\\\\") ||
    /^[a-zA-Z]:[\\/]/u.test(trimmed)
  );
}

function uniquePaths(paths: readonly string[]): string[] {
  const seen = new Set<string>();
  const ordered: string[] = [];
  for (const path of paths) {
    const trimmed = path.trim();
    if (trimmed === "" || seen.has(trimmed)) continue;
    seen.add(trimmed);
    ordered.push(trimmed);
  }
  return ordered;
}

function normalizePath(value: string): string {
  return value.trim().replace(/\\/gu, "/").toLowerCase();
}

function recentMatches(path: string, query: string): boolean {
  const wanted = normalizePath(query);
  if (wanted === "~" || wanted === "~/") return true;
  return scorePathMatch(wanted, normalizePath(path)) !== null;
}

/** The recent roots matching the field (all of them for an empty field),
 * then the machine's answer as it ranked it, without repeats. */
export function buildWorkingDirectorySuggestions(input: {
  recentPaths: readonly string[];
  machinePaths: readonly string[];
  query: string;
}): string[] {
  const query = input.query.trim();
  const recent = uniquePaths(input.recentPaths);
  if (query === "") return recent;
  return uniquePaths([
    ...recent.filter((path) => recentMatches(path, query)),
    ...input.machinePaths,
  ]);
}

/** The directory rows: a typed openable path not among the suggestions
 * first, then the suggestions. */
export function buildDirectoryOptions(input: {
  recentPaths: readonly string[];
  machinePaths: readonly string[];
  query: string;
}): DirectoryOption[] {
  const suggested = buildWorkingDirectorySuggestions(input);
  const suggestions = suggested.map((path): DirectoryOption => ({
    kind: "suggestion",
    path,
  }));
  const typed = input.query.trim();
  if (!isOpenableProjectPath(typed) || suggested.includes(typed)) {
    return suggestions;
  }
  return [{ kind: "path", path: typed }, ...suggestions];
}
