// A workspace tab's strip entry: its label and mark (split out of
// workspace.tsx at the page's 300-line budget when the Pull request tab
// came, TRACKER 430).

import { sessionName } from "../../data/titles";
import { tabKey, type WorkspaceTab } from "../../data/workspace/tab-layout";
import type { SessionId, SessionInfo, TerminalInfo } from "../../wire/proto";
import type { TabItem } from "./chrome-parts";
import type { TabWords } from "./words";

export interface TabFacts {
  session: (id: SessionId) => SessionInfo | undefined;
  terminals: readonly TerminalInfo[];
  dirty: ReadonlySet<string>;
}

export function tabItem(
  tab: WorkspaceTab,
  facts: TabFacts,
  words: TabWords,
): TabItem {
  const dirty = facts.dirty.has(tabKey(tab));
  switch (tab.kind) {
    case "agent": {
      const known = facts.session(tab.id);
      return {
        tab,
        label: known === undefined ? words.agent : sessionName(known),
        icon: known?.spec.mode === "terminal" ? "terminal" : "message",
        dirty,
      };
    }
    case "terminal":
      return {
        tab,
        label:
          facts.terminals.find((item) => item.id === tab.id)?.title ??
          words.terminal,
        icon: "terminal",
        dirty,
      };
    case "file":
      return {
        tab,
        label: tab.path.split("/").at(-1) ?? tab.path,
        icon: "file",
        dirty,
      };
    case "files":
      return { tab, label: words.files, icon: "folder", dirty };
    case "changes":
      return { tab, label: words.changes, icon: "git-compare", dirty };
    case "diff":
      return { tab, label: words.diff, icon: "git-compare", dirty };
    case "draft":
      return { tab, label: words.newTab, icon: "compose", dirty };
    case "pull":
      return {
        tab,
        label: words.pullRequest,
        icon: "git-pull-request",
        dirty,
      };
  }
}
