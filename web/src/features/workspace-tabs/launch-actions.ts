// The workspace's New tab entries (moved out of use-workspace.ts, move
// only, at the page's 300-line budget when the Pull request entry came,
// TRACKER 430): the agent, the terminal, Changes, Pull request, Diff and
// Files, each selecting its tab.

import type { WorkspaceTab } from "../../data/workspace/tab-layout";
import type { TabAction } from "./chrome-parts";
import type { useLaunchShortcuts } from "./launch-shortcuts";
import type { TabWords } from "./words";

export interface LaunchFacts {
  words: TabWords;
  select: (tab: WorkspaceTab) => void;
  newAgent: () => void;
  newTerminal: () => void;
  canCreateTerminal: boolean;
  /** The terminal entry says why it waits for an agent: below protocol 40
   * or a folder the page cannot name, with terminals on. */
  needsAgent: boolean;
  shortcuts: ReturnType<typeof useLaunchShortcuts>;
  /** The session has a pull request and its machine serves the pane. */
  pullOffered: boolean;
}

export function launchActions({
  words,
  select,
  newAgent,
  newTerminal,
  canCreateTerminal,
  needsAgent,
  shortcuts,
  pullOffered,
}: LaunchFacts): TabAction[] {
  return [
    {
      label: words.agent,
      icon: "compose",
      run: newAgent,
      ...(shortcuts.agent === null ? {} : { shortcut: shortcuts.agent }),
    },
    {
      label: words.terminal,
      icon: "terminal",
      disabled: !canCreateTerminal,
      // Only with no place: below protocol 40 (a workspace whose agents are
      // all archived), or a folder the page cannot name.
      ...(needsAgent ? { description: words.terminalNeedsAgent } : {}),
      run: newTerminal,
      ...(shortcuts.terminal === null ? {} : { shortcut: shortcuts.terminal }),
    },
    {
      label: words.changes,
      icon: "git-compare",
      run: () => {
        select({ kind: "changes" });
      },
    },
    // Paseo's pane sits after Changes; offered while the session has a
    // pull request read (TRACKER 430).
    ...(!pullOffered
      ? []
      : [
          {
            label: words.pullRequest,
            icon: "git-pull-request" as const,
            run: () => {
              select({ kind: "pull" });
            },
          },
        ]),
    {
      label: words.diff,
      icon: "git-compare",
      run: () => {
        select({ kind: "diff" });
      },
    },
    {
      label: words.files,
      icon: "folder",
      run: () => {
        select({ kind: "files" });
      },
    },
  ];
}
