// A session's plugin slash commands (ADR 0115, catalog and invoke), as
// Paseo's composer has them (`plugins/client-slash-commands/`,
// `composer/index.tsx` 1462-1473, 1725-1730): the session machine's plugin
// commands in the `/` list, and a submitted `/name args` that names one
// run as the plugin's method rather than sent to the agent.
//
// [Decision Log]
// - Purpose: Paseo's plugins add composer commands; Vessel's declare them
//   (a name, its help and the method that runs it).
// - Paseo's merge: the app's own commands first (the page has none), then
//   the plugins', then the agent's; an agent command with a plugin
//   command's name is left out, as Paseo's `mergeSlashCommandSources`.
// - The method's input is `{arguments}`, the text after the name; its
//   context the session (an agent context with its project and worktree).
//   Submitting clears the draft at once, as Paseo's does; a failure is a
//   toast. Text with files attached goes to the agent (Paseo's rule).

import { useMemo } from "react";

import { useFleet } from "../../context";
import {
  invokePlugin,
  type PluginSlashCommand,
  pluginCatalogs,
  pluginSlashCommands,
  resolveSlashCommand,
} from "../../data/plugin-catalog";
import { problemWords } from "../../data/plugins";
import { useStore } from "../../data/store";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type { AgentCommand, SessionInfo } from "../../wire/proto";

export interface PluginSlash {
  /** The agent's commands with the plugins' before them; `null` (the
   * agent has not said) stays `null` only when no plugin offers any, since
   * Paseo lists the plugins' commands while the agent's are unknown. */
  merge: (
    agent: readonly AgentCommand[] | null,
  ) => readonly AgentCommand[] | null;
  /** Run `text` when it names a plugin command; `true` when it did. */
  run: (text: string, hasAttachments: boolean) => boolean;
}

export function usePluginSlash(
  node: NodeKey,
  session: SessionInfo,
): PluginSlash {
  const fleet = useFleet();
  const catalogs = useStore(pluginCatalogs, (value) => value);
  const commands: readonly PluginSlashCommand[] = useMemo(
    () =>
      pluginSlashCommands(catalogs, node, {
        kind: "session",
        node,
        agent: session.id,
        workspace: session.workspace ?? null,
      }),
    [catalogs, node, session.id, session.workspace],
  );
  return useMemo(
    () => ({
      merge: (agent) => {
        if (commands.length === 0) return agent;
        const names = new Set(commands.map((command) => command.name));
        return [
          ...commands.map((command): AgentCommand => ({
            name: command.name,
            description: command.description,
            hint: command.argumentHint === "" ? null : command.argumentHint,
          })),
          ...(agent ?? []).filter((command) => !names.has(command.name)),
        ];
      },
      run: (text, hasAttachments) => {
        const resolved = resolveSlashCommand(text, hasAttachments, commands);
        if (resolved === null) return false;
        void invokePlugin(fleet, resolved.command.call(resolved.args)).then(
          (outcome) => {
            if (!outcome.ok) toast(problemWords(outcome), "error");
          },
        );
        return true;
      },
    }),
    [commands, fleet],
  );
}
