// Named script comparison at the approval boundary. A complete projection
// prevents an optional wire field from being silently omitted from comparison.

import type { ScriptEntry } from "../../wire/proto";

/** One script or service compared with the approved set, by name. */
export type ScriptLine =
  | { change: "same" | "added" | "removed"; script: ScriptEntry }
  | { change: "changed"; script: ScriptEntry; was: ScriptEntry };

/** Require a decision for every entry field, including future optional fields. */
function entryOf(script: ScriptEntry): Required<ScriptEntry> {
  return {
    name: script.name,
    kind: script.kind,
    command: script.command,
    dir: script.dir ?? null,
    port: script.port ?? null,
  };
}

/** Whether two entries of one name run the same thing in the same way. */
function sameScript(a: ScriptEntry, b: ScriptEntry): boolean {
  const before = entryOf(a);
  const after = entryOf(b);
  return (
    before.kind === after.kind &&
    before.command === after.command &&
    before.dir === after.dir &&
    before.port === after.port
  );
}

/** `after`'s scripts against `before`'s, by name, in name order. */
export function compareScripts(
  before: readonly ScriptEntry[],
  after: readonly ScriptEntry[],
): ScriptLine[] {
  const old = new Map(before.map((script) => [script.name, script]));
  const names = new Set(after.map((script) => script.name));
  const lines: ScriptLine[] = after.map((script) => {
    const was = old.get(script.name);
    if (was === undefined) {
      return { change: "added", script };
    }
    return sameScript(was, script)
      ? { change: "same", script }
      : { change: "changed", script, was };
  });
  for (const script of before) {
    if (!names.has(script.name)) {
      lines.push({ change: "removed", script });
    }
  }
  return lines.sort((a, b) =>
    a.script.name < b.script.name ? -1 : a.script.name > b.script.name ? 1 : 0,
  );
}

/** Refuse partial approval until a declaration can be displayed in full. */
export function hasDeclaredPorts(scripts: readonly ScriptEntry[]): boolean {
  return scripts.some((script) => entryOf(script).port !== null);
}
