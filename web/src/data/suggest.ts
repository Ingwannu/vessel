// The composer's one suggestion menu (milestone 11, Track A-page): `/`
// at the start of a prompt lists the agent's commands, `@` at the start
// of a word finds the session's files. What the caret is in, what is
// listed, in which order, and what choosing does to the text are decided
// here; the menu draws the result.
//
// [Decision Log]
// - Purpose: agent-orchestrator, GOAT and Paseo complete `/` and `@` in
//   their composers from one menu at the caret.
// - Constraint: a command is used by sending a prompt that starts with
//   `/name` (the node passes it to the agent as text), so `/` means a
//   command only at the very start. A file is named to the agent as text
//   too: `@path`, which every agent Vessel runs reads as a mention.
// - Choice: plain text in the box, not a token: the prompt the person
//   sees is the prompt the agent gets, and a draft the bridge keeps is
//   text. The query is the word up to the caret; choosing replaces the
//   word and leaves a space after it.
// - Cost: a file name with a space is written in quotes (`@"a b.txt"`),
//   which is what Claude Code and Codex read; another agent may not.

import type { AgentCommand, FoundFile } from "../wire/proto";

/** What the caret is in, when it is something to complete. */
export type Trigger =
  | { kind: "command"; query: string; start: number; end: number }
  | { kind: "file"; query: string; start: number; end: number };

/** The most rows a menu shows. */
export const MAX_ROWS = 50;

/** The trigger at `caret` in `text`, if any: the word up to the caret
 * starts with `/` at the prompt's start, or with `@` after a space or a
 * line's start. */
export function triggerAt(text: string, caret: number): Trigger | null {
  let start = caret;
  while (start > 0 && !/\s/.test(text.charAt(start - 1))) {
    start -= 1;
  }
  const word = text.slice(start, caret);
  // The word goes on past the caret until a space: choosing replaces it.
  let end = caret;
  while (end < text.length && !/\s/.test(text.charAt(end))) {
    end += 1;
  }
  if (word.startsWith("/") && start === 0) {
    return { kind: "command", query: word.slice(1), start, end };
  }
  if (word.startsWith("@")) {
    return { kind: "file", query: word.slice(1), start, end };
  }
  return null;
}

/** The commands matching `query`: names that start with it first, then
 * names that hold it, then descriptions that do, each in the agent's
 * order; at most `MAX_ROWS`. */
export function matchCommands(
  commands: readonly AgentCommand[],
  query: string,
): AgentCommand[] {
  const wanted = query.toLowerCase();
  const tiers: AgentCommand[][] = [[], [], []];
  for (const command of commands) {
    const name = command.name.toLowerCase();
    if (name.startsWith(wanted)) {
      tiers[0]?.push(command);
    } else if (name.includes(wanted)) {
      tiers[1]?.push(command);
    } else if (command.description.toLowerCase().includes(wanted)) {
      tiers[2]?.push(command);
    }
  }
  return tiers.flat().slice(0, MAX_ROWS);
}

/** Text with `trigger`'s word replaced by `word` and a space, and where
 * the caret goes after. */
export interface Applied {
  text: string;
  caret: number;
}

function replaceWord(text: string, trigger: Trigger, word: string): Applied {
  const after = text.slice(trigger.end);
  const spaced = after.startsWith(" ") ? word : `${word} `;
  return {
    text: text.slice(0, trigger.start) + spaced + after,
    caret: trigger.start + spaced.length + (after.startsWith(" ") ? 1 : 0),
  };
}

/** Choose command `name` for `trigger`. */
export function applyCommand(
  text: string,
  trigger: Trigger,
  name: string,
): Applied {
  return replaceWord(text, trigger, `/${name}`);
}

/** File `path` as the composer's `@` names it: quoted when it holds a
 * space or a quote. */
export function fileMention(path: string): string {
  return /\s|"/.test(path) ? `@"${path.replaceAll('"', '\\"')}"` : `@${path}`;
}

/** Choose file `path` for `trigger`. */
export function applyFile(
  text: string,
  trigger: Trigger,
  path: string,
): Applied {
  return replaceWord(text, trigger, fileMention(path));
}

/** `draft` with file `path` named after what is in it (the Explorer's Add
 * to chat), a space either side so it stays a word of its own. */
export function withMention(draft: string, path: string): string {
  const mention = `${fileMention(path)} `;
  if (draft.trim() === "") return mention;
  return /\s$/.test(draft) ? `${draft}${mention}` : `${draft} ${mention}`;
}

/** A found file's path split for drawing: the folder (dimmed) and the
 * name, with the characters the search matched marked. */
export interface PathParts {
  folder: string;
  name: string;
  /** Positions in `name` the query matched. */
  marks: ReadonlySet<number>;
}

/** `file`'s path as the menu draws it. Positions are code points of the
 * whole path (the node's `positions`). */
export function pathParts(file: FoundFile): PathParts {
  const characters = Array.from(file.path);
  const cut = characters.lastIndexOf("/") + 1;
  const marks = new Set<number>();
  for (const position of file.positions) {
    if (position >= cut) {
      marks.add(position - cut);
    }
  }
  return {
    folder: characters.slice(0, cut).join(""),
    name: characters.slice(cut).join(""),
    marks,
  };
}
