import { useState } from "react";

import { useFleet } from "../../context";
import { reloadSession, renameSession } from "../../data/calls";
import { canReload } from "../../data/reload";
import type { WorkspaceTab } from "../../data/workspace/tab-layout";
import { renameWorkspaceTerminal } from "../../data/workspace/terminals";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type {
  SessionId,
  SessionInfo,
  TerminalId,
  TerminalInfo,
} from "../../wire/proto";
import type { TabAction, TabItem } from "./chrome-parts";
import { useCloseTabs } from "./use-close-tabs";
import type { TabWords } from "./words";

interface TabLifecycle {
  confirmation: ReturnType<typeof useCloseTabs>["confirmation"];
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
  const closing = useCloseTabs(input);
  const [renaming, setRenaming] = useState<TabItem | null>(null);

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
            run: () => {
              copy(
                `vessel${node === "_here" ? "" : ` --node ${node}`} session resume ${tab.id}`,
                words.agent,
              );
            },
          },
          {
            label: words.copyAgent,
            icon: "copy",
            run: () => {
              copy(tab.id, item.label);
            },
          },
          {
            label: words.rename,
            icon: "edit",
            run: () => {
              setRenaming(item);
            },
          },
          {
            label: words.reload,
            tooltip: words.reloadHint,
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
            run: () => {
              copy(tab.id, item.label);
            },
          },
          {
            label: words.rename,
            icon: "edit",
            run: () => {
              setRenaming(item);
            },
          },
        ];
      case "file":
        return [
          {
            label: words.copyPath,
            icon: "copy",
            run: () => {
              copy(tab.path, words.files);
            },
          },
        ];
      case "changes":
      case "diff":
      case "files":
      case "draft":
      case "pull":
        return [];
    }
  };

  return {
    confirmation: closing.confirmation,
    cancel: closing.cancel,
    confirm: closing.confirm,
    askClose: (item) => {
      closing.close({ kind: "single", tab: item.tab });
    },
    bulk: (tabs, title) => {
      closing.close({ kind: "bulk", tabs, title });
    },
    actions,
    renaming,
    cancelRename: () => {
      setRenaming(null);
    },
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
