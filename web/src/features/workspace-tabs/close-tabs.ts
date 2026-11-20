// Close the confirmed targets through their owners. Each confirmed
// result changes layout independently so partial failure is retryable.

import { readSession } from "../../data/calls";
import { markSession } from "../../data/conversation-calls";
import type { Fleet } from "../../data/fleet";
import { nodeError } from "../../data/workspace/root";
import type { WorkspaceTab } from "../../data/workspace/tab-layout";
import { closeWorkspaceTerminal } from "../../data/workspace/terminals";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type { SessionId, SessionInfo, TerminalId } from "../../wire/proto";

export function childAgent(session: SessionInfo): boolean {
  return session.created_by.kind === "agent" && session.detach_receipt == null;
}

interface CloseActions {
  node: NodeKey;
  session: (id: SessionId) => SessionInfo | undefined;
  remove: (tab: WorkspaceTab) => void;
  forgetTerminal: (id: TerminalId) => void;
}

export async function applyClose(
  fleet: Fleet,
  input: CloseActions,
  tabs: readonly WorkspaceTab[],
): Promise<void> {
  const { node } = input;
  // Archive may also stop session-owned terminals. Do the explicit
  // terminal closes first, so their outcomes can be reported separately.
  const ordered = [
    ...tabs.filter((tab) => tab.kind === "terminal"),
    ...tabs.filter((tab) => tab.kind !== "terminal"),
  ];
  for (const tab of ordered) {
    if (tab.kind === "agent") {
      const known = input.session(tab.id);
      const asked =
        known === undefined ? await readSession(fleet, node, tab.id) : null;
      if (asked !== null && !asked.ok) {
        toast(asked.problem.message, "error");
        continue;
      }
      const agent = known ?? (asked?.ok === true ? asked.value : undefined);
      if (
        agent !== undefined &&
        !childAgent(agent) &&
        agent.marks?.archived_at == null
      ) {
        const result = await markSession(fleet, node, tab.id, {
          mark: "archive",
        });
        if (!result.ok) {
          toast(result.problem.message, "error");
          continue;
        }
        fleet.putSession(node, result.value);
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
