// One list of a repository's commands in the approval dialog (setup, or
// teardown), each command as it will run, in order, marked when it is new,
// edited or gone since the set approved last; an edited one shows what it
// was beneath it.

import "./projects.css";

import type { CommandLine } from "../../data/workspace/command-changes";
import { Badge } from "../../ui/kit";

/** The mark in the gutter, as a diff draws it. */
function mark(change: CommandLine["change"]): string {
  switch (change) {
    case "added":
      return "+";
    case "removed":
      return "−";
    case "changed":
      return "~";
    case "same":
      return "";
  }
}

/** The badge after a line, or none for a line as approved. */
export function ChangeBadge({ change }: { change: CommandLine["change"] }) {
  switch (change) {
    case "same":
      return null;
    case "added":
      return <Badge tone="warn">new</Badge>;
    case "changed":
      return <Badge tone="warn">changed</Badge>;
    case "removed":
      return <Badge>removed</Badge>;
  }
}

/** The gutter's mark for a line. */
export function ChangeMark({ change }: { change: CommandLine["change"] }) {
  return (
    <span className="command-mark" aria-hidden="true">
      {mark(change)}
    </span>
  );
}

export function CommandList({
  title,
  lines,
}: {
  title: string;
  lines: readonly CommandLine[];
}) {
  return (
    <section className="commands" aria-label={title}>
      <h3>{title}</h3>
      {lines.length === 0 ? (
        <p className="commands-none">None.</p>
      ) : (
        <ol className="commands-list">
          {lines.map((line, index) => (
            <li
              key={`${String(index)}:${line.command}`}
              className={`command ${line.change}`}
            >
              <ChangeMark change={line.change} />
              <span className="command-text">
                <code>{line.command}</code>
                {line.change === "changed" ? (
                  <span className="command-was">
                    was <code>{line.was}</code>
                  </span>
                ) : null}
              </span>
              <ChangeBadge change={line.change} />
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
