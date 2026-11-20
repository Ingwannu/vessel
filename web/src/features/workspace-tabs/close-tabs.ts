// Discover the node-side effects before the user confirms. Each accepted
// mutation changes layout independently so a partial failure stays retryable.

import { readSession } from "../../data/calls";
import { markSession } from "../../data/conversation-calls";
import type { Fleet } from "../../data/fleet";
import { nodeError } from "../../data/workspace/root";
import type { WorkspaceTab } from "../../data/workspace/tab-layout";
import {
  closeWorkspaceTerminal,
  TERMINAL_PROTOCOL,
  workspaceTerminals,
} from "../../data/workspace/terminals";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type {
  SessionId,
  SessionInfo,
  TerminalId,
  TerminalInfo,
} from "../../wire/proto";

export function childAgent(session: SessionInfo): boolean {
  return session.created_by.kind === "agent" && session.detach_receipt == null;
}

export function isLiveTerminal(terminal: TerminalInfo): boolean {
  return (
    terminal.status.state === "starting" || terminal.status.state === "running"
  );
}

/** The live shells an archive of `agents` ends: only those placed beside
 * one of them. A worktree's or a folder's shells (ADR 0121, protocol 40)
 * belong to the workspace and outlive every archive, so the archive
 * warning never counts them; a project's never were counted. */
export function endedByArchive(
  listed: readonly TerminalInfo[],
  agents: ReadonlyMap<SessionId, SessionInfo>,
): TerminalId[] {
  return listed
    .filter(
      (terminal) =>
        terminal.place.kind === "session" &&
        agents.has(terminal.place.session) &&
        isLiveTerminal(terminal),
    )
    .map((terminal) => terminal.id);
}

export interface CloseActions {
  node: NodeKey;
  protocol: number | null;
  session: (id: SessionId) => SessionInfo | undefined;
  remove: (tab: WorkspaceTab) => void;
  forgetTerminal: (id: TerminalId) => void;
}

export interface CloseImpact {
  agents: ReadonlyMap<SessionId, SessionInfo>;
  terminals: ReadonlySet<TerminalId>;
  passive: number;
}

export async function inspectClose(
  fleet: Fleet,
  input: CloseActions,
  tabs: readonly WorkspaceTab[],
): Promise<CloseImpact | null> {
  const agents = new Map<SessionId, SessionInfo>();
  const terminals = new Set<TerminalId>();
  for (const tab of tabs) {
    if (tab.kind === "terminal") {
      terminals.add(tab.id);
    }
    if (tab.kind !== "agent") {
      continue;
    }
    let session = input.session(tab.id);
    if (session === undefined) {
      const result = await readSession(fleet, input.node, tab.id);
      if (!result.ok) {
        if (nodeError(result.problem) === "not_found") {
          continue;
        }
        toast(result.problem.message, "error");
        return null;
      }
      session = result.value;
    }
    if (!childAgent(session) && session.marks?.archived_at == null) {
      agents.set(tab.id, session);
    }
  }

  const passive = tabs.length - agents.size - terminals.size;
  // The registry's bounded list holds all live terminals. One read also
  // counts script/service terminals and shells the tab strip never mounted.
  if (
    agents.size > 0 &&
    (input.protocol === null || input.protocol >= TERMINAL_PROTOCOL)
  ) {
    const result = await workspaceTerminals(fleet, input.node, null);
    if (!result.ok) {
      toast(result.problem.message, "error");
      return null;
    }
    for (const id of endedByArchive(result.value.terminals, agents)) {
      terminals.add(id);
    }
  }
  return { agents, terminals, passive };
}

export function impactGrew(before: CloseImpact, latest: CloseImpact): boolean {
  return (
    [...latest.agents.keys()].some((id) => !before.agents.has(id)) ||
    [...latest.terminals].some((id) => !before.terminals.has(id))
  );
}

export async function applyClose(
  fleet: Fleet,
  input: CloseActions,
  tabs: readonly WorkspaceTab[],
  impact: CloseImpact,
): Promise<void> {
  const { node } = input;
  // Explicit terminal closes go first so their outcomes are observable
  // before an agent archive stops the remaining terminals beside it.
  const ordered = [
    ...tabs.filter((tab) => tab.kind === "terminal"),
    ...tabs.filter((tab) => tab.kind !== "terminal"),
  ];
  for (const tab of ordered) {
    if (tab.kind === "agent" && impact.agents.has(tab.id)) {
      const result = await markSession(fleet, node, tab.id, {
        mark: "archive",
      });
      if (result.ok) {
        fleet.putSession(node, result.value);
      } else if (nodeError(result.problem) !== "not_found") {
        toast(result.problem.message, "error");
        continue;
      }
    }
    if (tab.kind === "terminal") {
      const result = await closeWorkspaceTerminal(fleet, node, tab.id);
      if (!result.ok && nodeError(result.problem) !== "not_found") {
        toast(result.problem.message, "error");
        continue;
      }
      input.forgetTerminal(tab.id);
    }
    input.remove(tab);
  }
}
