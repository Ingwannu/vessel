import { useRef, useState } from "react";

import { useFleet } from "../../context";
import { readSession, reloadSession, renameSession } from "../../data/calls";
import { markSession } from "../../data/conversation-calls";
import { isLive } from "../../data/format";
import { canReload } from "../../data/reload";
import type { WorkspaceTab } from "../../data/workspace/tab-layout";
import { nodeError } from "../../data/workspace/root";
import {
  closeWorkspaceTerminal,
  renameWorkspaceTerminal,
} from "../../data/workspace/terminals";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type {
  SessionId,
  SessionInfo,
  TerminalId,
  TerminalInfo,
} from "../../wire/proto";
import type { TabAction, TabItem } from "./chrome-parts";
import type { TabWords } from "./words";

interface Confirmation {
  title: string;
  message: string;
  label: string;
  tabs: readonly WorkspaceTab[];
}

export function childAgent(session: SessionInfo): boolean {
  return session.created_by.kind === "agent" && session.detach_receipt == null;
}

interface TabLifecycle {
  confirmation: Confirmation | null;
  cancel: () => void;
  confirm: () => void;
  askClose: (item: TabItem) => void;
  bulk: (tabs: readonly WorkspaceTab[], title: string) => void;
  actions: (item: TabItem) => readonly TabAction[];
  renaming: TabItem | null;
  cancelRename: () => void;
  rename: (title: string) => Promise<string | null>;
}

export function useTabLifecycle(input: {
  node: NodeKey;
  protocol: number | null;
  words: TabWords;
  session: (id: SessionId) => SessionInfo | undefined;
  dirty: (tab: WorkspaceTab) => boolean;
  remove: (tab: WorkspaceTab) => void;
  forgetTerminal: (id: TerminalId) => void;
  renamedTerminal: (terminal: TerminalInfo) => void;
}): TabLifecycle {
  const { node, words } = input;
  const fleet = useFleet();
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [renaming, setRenaming] = useState<TabItem | null>(null);
  const closing = useRef(false);

  const closeTabs = async (tabs: readonly WorkspaceTab[]) => {
    if (closing.current) {
      return;
    }
    closing.current = true;
    try {
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
    } finally {
      closing.current = false;
    }
  };

  const askClose = (item: TabItem) => {
    if (closing.current) {
      return;
    }
    const tab = item.tab;
    if (tab.kind === "terminal") {
      setConfirmation({
        title: words.terminalTitle,
        message: words.terminalBody,
        label: words.close,
        tabs: [tab],
      });
      return;
    }
    if (tab.kind === "agent") {
      const agent = input.session(tab.id);
      if (
        agent === undefined ||
        (!childAgent(agent) &&
          isLive(agent) &&
          agent.marks?.archived_at == null)
      ) {
        setConfirmation({
          title: words.agentTitle,
          message: words.agentBody,
          label: words.archive,
          tabs: [tab],
        });
        return;
      }
    }
    if (input.dirty(tab)) {
      setConfirmation({
        title: words.fileTitle,
        message: words.fileBody,
        label: words.close,
        tabs: [tab],
      });
      return;
    }
    void closeTabs([tab]);
  };

  const bulk = (tabs: readonly WorkspaceTab[], title: string) => {
    if (closing.current || tabs.length === 0) {
      return;
    }
    const agents = tabs.filter((tab) => {
      if (tab.kind !== "agent") {
        return false;
      }
      const agent = input.session(tab.id);
      return (
        agent === undefined ||
        (!childAgent(agent) && agent.marks?.archived_at == null)
      );
    }).length;
    const terminals = tabs.filter((tab) => tab.kind === "terminal").length;
    setConfirmation({
      title,
      label: words.close,
      message: words.bulk(
        agents,
        terminals,
        tabs.length - agents - terminals,
        tabs.filter(input.dirty).length,
      ),
      tabs,
    });
  };

  const copy = (text: string, label: string) => {
    void navigator.clipboard.writeText(text).then(
      () => toast(`${words.copied} · ${label}`),
      () => toast(words.unavailable, "error"),
    );
  };

  const reloadable = (id: SessionId) => {
    const agent = input.session(id);
    return agent !== undefined && canReload(agent, input.protocol);
  };

  const actions = (item: TabItem): readonly TabAction[] => {
    const tab = item.tab;
    switch (tab.kind) {
      case "agent":
        return [
          {
            label: words.copyResume,
            icon: "copy",
            disabled: input.session(tab.id)?.spec.mode !== "structured",
            run: () =>
              copy(
                `vessel${node === "_here" ? "" : ` --node ${node}`} session resume ${tab.id}`,
                words.agent,
              ),
          },
          {
            label: words.copyAgent,
            icon: "copy",
            run: () => copy(tab.id, item.label),
          },
          { label: words.rename, icon: "edit", run: () => setRenaming(item) },
          {
            label: words.reload,
            icon: "refresh",
            disabled: !reloadable(tab.id),
            run: () => {
              void reloadSession(fleet, node, tab.id).then((result) => {
                if (result.ok) {
                  fleet.putSession(node, result.value);
                } else {
                  toast(result.problem.message, "error");
                }
              });
            },
          },
        ];
      case "terminal":
        return [
          {
            label: words.copyTerminal,
            icon: "copy",
            run: () => copy(tab.id, item.label),
          },
          { label: words.rename, icon: "edit", run: () => setRenaming(item) },
        ];
      case "file":
        return [
          {
            label: words.copyPath,
            icon: "copy",
            run: () => copy(tab.path, words.files),
          },
        ];
      case "changes":
      case "diff":
      case "files":
      case "draft":
        return [];
    }
  };

  return {
    confirmation,
    cancel: () => setConfirmation(null),
    confirm: () => {
      if (confirmation !== null) {
        setConfirmation(null);
        void closeTabs(confirmation.tabs);
      }
    },
    askClose,
    bulk,
    actions,
    renaming,
    cancelRename: () => setRenaming(null),
    rename: async (title: string): Promise<string | null> => {
      const tab = renaming?.tab;
      if (tab?.kind === "agent") {
        const result = await renameSession(fleet, node, tab.id, title);
        if (!result.ok) {
          return result.problem.message;
        }
        fleet.putSession(node, result.value);
      } else if (tab?.kind === "terminal") {
        const result = await renameWorkspaceTerminal(
          fleet,
          node,
          tab.id,
          title,
        );
        if (!result.ok) {
          return result.problem.message;
        }
        input.renamedTerminal(result.value);
      }
      setRenaming(null);
      return null;
    },
  };
}
