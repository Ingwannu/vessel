// A set's scripts and services in the approval dialog (ADR 0031: the
// digest covers them, so approving a set approves them too). Every one is
// shown, by name, with what it runs and where; one gone since the set
// approved last is shown struck through, and an edited one says what
// changed and what it ran before.

import "./projects.css";

import type { ScriptLine } from "../../data/workspace/command-changes";
import {
  scriptChangeWords,
  scriptKindWord,
} from "../../data/workspace/command-words";
import { ChangeBadge, ChangeMark } from "./command-list";

export function ScriptList({ lines }: { lines: readonly ScriptLine[] }) {
  return (
    <section className="commands" aria-label="Scripts and services">
      <h3>Scripts and services</h3>
      <ol className="commands-list">
        {lines.map((line) => (
          <li
            key={`${line.change}:${line.script.name}`}
            className={`command script ${line.change}`}
          >
            <ChangeMark change={line.change} />
            <span className="command-text">
              <span className="script-head">
                <span className="script-name">{line.script.name}</span>
                <span className="script-kind">
                  {scriptKindWord(line.script.kind)}
                  {line.script.dir === undefined || line.script.dir === null
                    ? null
                    : ` in ${line.script.dir}/`}
                </span>
              </span>
              <code>{line.script.command}</code>
              {line.change === "changed" ? (
                <span className="command-was">
                  Changed: {scriptChangeWords(line.was, line.script)}. Before:{" "}
                  <code>{line.was.command}</code>
                  {line.was.kind === line.script.kind
                    ? null
                    : `, as a ${scriptKindWord(line.was.kind)}`}
                  {(line.was.dir ?? null) === null
                    ? null
                    : `, in ${line.was.dir ?? ""}/`}
                </span>
              ) : null}
            </span>
            <ChangeBadge change={line.change} />
          </li>
        ))}
      </ol>
    </section>
  );
}
