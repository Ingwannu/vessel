// The same registered actions drive the key listener and the menu hints.
// They exist only while a workspace's launcher is mounted.

import { useShortcut } from "../shortcuts/handlers";
import { useShortcutWords } from "../shortcuts/use-keys";

interface LaunchShortcuts {
  agent: string | null;
  terminal: string | null;
}

export function useLaunchShortcuts(
  launchAgent: () => void,
  launchTerminal: () => void,
  terminalEnabled: boolean,
): LaunchShortcuts {
  useShortcut("workspace-tab-target-agent", () => {
    launchAgent();
    return true;
  });
  useShortcut(
    "workspace-terminal-new",
    () => {
      launchTerminal();
      return true;
    },
    terminalEnabled,
  );
  return {
    agent: useShortcutWords("workspace-tab-target-agent"),
    terminal: useShortcutWords("workspace-terminal-new"),
  };
}
