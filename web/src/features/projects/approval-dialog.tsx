// A repository's commands, shown before they may run (ADR 0027): exactly
// what will run, in order, and what changed from the set approved for the
// project's folder, then Approve (the digest shown is the one sent), or
// start without them. No reference asks this (each runs a repository's
// setup unasked, or only commands typed into its own page); Vessel does,
// because a repository's file is someone else's code.

import "./projects.css";

import {
  type CommandLine,
  type Compared,
  type ListedCommands,
} from "../../data/workspace/command-changes";
import { Badge, Banner, Button, Dialog } from "../../ui/kit";

function CommandList({
  title,
  commands,
  compared,
}: {
  title: string;
  commands: readonly string[];
  compared: readonly CommandLine[] | null;
}) {
  const lines: readonly CommandLine[] =
    compared ?? commands.map((command) => ({ change: "same", command }));
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
              <span className="command-mark" aria-hidden="true">
                {line.change === "added"
                  ? "+"
                  : line.change === "removed"
                    ? "−"
                    : ""}
              </span>
              <code>{line.command}</code>
              {line.change === "same" ? null : (
                <Badge tone={line.change === "added" ? "warn" : "neutral"}>
                  {line.change === "added" ? "new" : "removed"}
                </Badge>
              )}
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

export function ApprovalDialog({
  commands,
  compared,
  where,
  busy,
  onApprove,
  onSkip,
  onClose,
}: {
  commands: ListedCommands;
  compared: Compared;
  /** Where they would run, in words ("in a new worktree of shop on
   * laptop"). */
  where: string;
  busy: boolean;
  onApprove: () => void;
  /** Go on without them (a new worktree without setup); none when there
   * is nothing to go on to. */
  onSkip?: () => void;
  onClose: () => void;
}) {
  return (
    <Dialog
      title="Run this project's commands?"
      size="lg"
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          {onSkip === undefined ? null : (
            <Button onClick={onSkip}>Start without setup</Button>
          )}
          <Button variant="primary" busy={busy} onClick={onApprove}>
            Approve and run
          </Button>
        </>
      }
    >
      <p className="dialog-copy">
        The repository&apos;s <code>vessel.toml</code> names these commands.
        They run {where}, as you, with your environment. Approving covers this
        exact set; if any command changes, you are asked again.
      </p>
      {compared.kind === "changed" ? (
        <p className="commands-note">
          Compared with the set approved for the project&apos;s folder: new and
          removed commands are marked.
        </p>
      ) : (
        <p className="commands-note">
          No approved set of this project is known to compare with.
        </p>
      )}
      <CommandList
        title="Setup"
        commands={commands.setup}
        compared={compared.kind === "changed" ? compared.setup : null}
      />
      <CommandList
        title="Teardown"
        commands={commands.teardown}
        compared={compared.kind === "changed" ? compared.teardown : null}
      />
      <Banner>
        A command that runs a script of the repository (<code>./setup.sh</code>,{" "}
        <code>npm ci</code> running package scripts) runs whatever that script
        says on that branch.
      </Banner>
      <p className="commands-digest">Digest {commands.digest}</p>
    </Dialog>
  );
}
