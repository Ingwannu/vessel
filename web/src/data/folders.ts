// The folder field's suggestions on the new-session page: the folders
// sessions already ran in on that machine, the latest first.
//
// The plan rules out listing a machine's disk (no file browser on a
// node); what the page knows is where sessions ran, which is also where a
// person is most likely to start the next one. The field takes anything
// typed, so a folder never used before is one keystroke away.

import type { SessionInfo } from "../wire/proto";
import { ago } from "./format";

/** One suggestion: the folder, and when a session last started there. */
export interface FolderSuggestion {
  value: string;
  detail: string;
}

/** How many folders the field suggests. */
export const MAX_FOLDERS = 12;

/** The folders `sessions` ran in, each once, the latest first. */
export function folderSuggestions(
  sessions: readonly SessionInfo[],
  now: number = Date.now(),
): FolderSuggestion[] {
  const latest = new Map<string, string>();
  for (const session of sessions) {
    const cwd = session.spec.cwd;
    if (cwd === undefined || cwd === null || cwd === "") {
      continue;
    }
    const seen = latest.get(cwd);
    if (seen === undefined || session.created_at > seen) {
      latest.set(cwd, session.created_at);
    }
  }
  return [...latest.entries()]
    .sort(([, a], [, b]) => b.localeCompare(a))
    .slice(0, MAX_FOLDERS)
    .map(([value, at]) => ({ value, detail: `${ago(at, now)} ago` }));
}
