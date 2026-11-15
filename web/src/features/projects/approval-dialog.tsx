// A repository's commands, shown before they may run (ADR 0027): exactly
// what will run, in order (setup, teardown, and the scripts and services
// the digest covers, ADR 0031), and what changed since the set approved
// last, then Approve (the digest shown is the one sent), or start without
// them. No reference asks this (each runs a repository's setup unasked,
// or only commands typed into its own page); Vessel does, because a
// repository's file is someone else's code.
//
// The dialog never offers to approve a set without every script in view:
// the scripts are drawn from the very set whose digest Approve sends, in
// full (at most 32, the file's bound), not summarised.

import "./projects.css";

import {
  approvalCoverage,
  type ApprovalRefusal,
  type CommandLine,
  type Compared,
  type ListedCommands,
  type ScriptLine,
} from "../../data/workspace/command-changes";
import { comparedWords } from "../../data/workspace/command-words";
import { Banner, Button, Dialog } from "../../ui/kit";
import { CommandList } from "./command-list";
import { ScriptList } from "./script-list";

/** Each list as it is when nothing is known to compare it with. */
function asListed(commands: readonly string[]): CommandLine[] {
  return commands.map((command) => ({ change: "same", command }));
}

/** Keep this screen exhaustive even when a new approved field is optional. */
function listedForDisplay(commands: ListedCommands): Required<ListedCommands> {
  // Enumerate the wire type, including optional fields: a future addition
  // must fail typecheck instead of silently approving unseen commands.
  return {
    state: commands.state,
    setup: commands.setup,
    teardown: commands.teardown,
    digest: commands.digest,
    approved: commands.approved,
    scripts: commands.scripts ?? [],
    terminals: commands.terminals ?? [],
    service_ports: commands.service_ports ?? null,
    last_approved: commands.last_approved ?? null,
  };
}

interface ApprovalDialogProps {
  commands: ListedCommands;
  compared: Compared;
  /** Where setup and teardown would run, in words ("in a new worktree of
   * shop on laptop"). */
  where: string;
  busy: boolean;
  onApprove: () => void;
  /** Go on without them (a new worktree without setup); none when there
   * is nothing to go on to. */
  onSkip?: () => void;
  onClose: () => void;
}

/** Offer approval only when every command covered by the digest is visible. */
export function ApprovalDialog({
  commands,
  compared,
  where,
  busy,
  onApprove,
  onSkip,
  onClose,
}: ApprovalDialogProps) {
  const shown = listedForDisplay(commands);
  const coverage = approvalCoverage(shown);
  const refusal: ApprovalRefusal | null =
    coverage !== "complete"
      ? { kind: coverage }
      : compared.kind === "unsupported_terminals" ||
          compared.kind === "unsupported_ports"
        ? compared
        : null;
  const refused = refusal !== null;
  return (
    <Dialog
      title={
        refused
          ? "This set cannot be approved here"
          : "Run this project's commands?"
      }
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
          {refused ? null : (
            <Button variant="primary" busy={busy} onClick={onApprove}>
              Approve and run
            </Button>
          )}
        </>
      }
    >
      {refusal !== null ? (
        <p className="dialog-copy">{comparedWords(refusal)}</p>
      ) : (
        <ApprovalContents shown={shown} compared={compared} where={where} />
      )}
    </Dialog>
  );
}

/** Keep the complete visible command set together, after the refusal gate. */
function ApprovalContents({
  shown,
  compared,
  where,
}: {
  shown: Required<ListedCommands>;
  compared: Compared;
  where: string;
}) {
  const changed = compared.kind === "changed" ? compared : null;
  const scripts: readonly ScriptLine[] =
    changed?.scripts ??
    shown.scripts.map((script) => ({ change: "same", script }));
  return (
    <>
      <p className="dialog-copy">
        The repository&apos;s <code>vessel.toml</code> names these commands.
        Setup and teardown run {where}
        {scripts.length === 0
          ? ""
          : "; a script or service runs when someone starts it"}
        , as you, with your environment. Approving covers this exact set; if any
        command changes, you are asked again.
      </p>
      <p className="commands-note">{comparedWords(compared)}</p>
      <CommandList
        title="Setup"
        lines={changed?.setup ?? asListed(shown.setup)}
      />
      <CommandList
        title="Teardown"
        lines={changed?.teardown ?? asListed(shown.teardown)}
      />
      {scripts.length === 0 ? null : <ScriptList lines={scripts} />}
      <Banner>
        A command that runs a script of the repository (<code>./setup.sh</code>,{" "}
        <code>npm ci</code> running package scripts) runs whatever that script
        says on that branch.
      </Banner>
      <p className="commands-digest">Digest {shown.digest}</p>
    </>
  );
}
