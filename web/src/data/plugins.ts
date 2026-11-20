// A machine's plugins (Settings › Plugins, ADR 0115): the calls, the
// page's state, the install field and the words, apart from drawing.
//
// [Decision Log]
// - Purpose: Paseo's host Plugins page
//   (`packages/app/src/screens/settings/plugins-page.tsx`,
//   `plugins-page-state.ts`, `plugin-install-form-model.ts`) lists a host's
//   plugins, switches plugins on for the host, installs from a source,
//   enables, disables, reloads and removes one, and shows its logs. The
//   page does the same over the node's `plugin.*` calls.
// - Existing pieces: the bridge's typed calls and `attempt`/`Outcome`, as
//   Settings › Devices uses them; the node's settings for the switch
//   (`node.settings` / `node.set_settings`, as Paseo patches its daemon
//   config's `pluginsEnabled`).
// - Choice: the decisions (which state the page is in, whether the field
//   can be sent, what a status or a change is called) are plain functions
//   here, tested on their own; the component only draws them.
// - Paseo's words, with "host" read as "machine" (Vessel's word, as ADR
//   0113's sheet). Stated differences: the trust notice does not say
//   plugins run inside the app (Vessel draws plugin data with its own
//   controls, ADR 0079), and the source field says a directory, the only
//   source this protocol installs (ADR 0115).

import type { NodeKey } from "../wire/bridge";
import {
  PLUGINS_PROTOCOL,
  type PluginGeneration,
  type PluginListItem,
  type PluginLogs,
  type PluginStatus,
} from "../wire/proto";
import type { Fleet } from "./fleet";
import { attempt, nextStep, type Outcome } from "./outcome";

/** The words of the section, Paseo's (`settings.plugins.*`). */
export const PLUGIN_WORDS = {
  title: "Plugins",
  trustedTitle: "Plugins are trusted code",
  trustedDescription:
    "They run on this machine without sandboxing. Install only code you trust.",
  globalTitle: "Enable plugins",
  globalHint: "Global switch for every configured plugin",
  sourceLabel: "Plugin source",
  sourcePlaceholder: "Directory on this machine, ~/… or /…",
  install: "Install plugin",
  installing: "Installing…",
  offlineTitle: "Plugin host is offline",
  offlineDescription: "Reconnect to this machine to manage its plugins.",
  updateTitle: "Update this machine to manage plugins",
  loading: "Loading plugins…",
  errorTitle: "Unable to load plugins",
  retry: "Retry",
  empty: "No plugins configured",
  removeConfirmMessage:
    "This removes the configuration. The source directory is not deleted.",
  logsAction: "Logs",
  logsLoading: "Loading logs…",
  logsRefresh: "Refresh",
  logsRefreshing: "Refreshing…",
  logsErrorTitle: "Unable to load plugin logs",
  logsEmpty: "No plugin output yet",
  reload: "Reload",
  reloading: "Reloading…",
  remove: "Remove",
  removing: "Removing…",
  enable: "Enable",
  disable: "Disable",
  globalEnabled: "Plugins enabled",
  globalDisabled: "Plugins disabled",
} as const;

/** Paseo's `resolvePluginPageState`. */
export type PluginPageState =
  "offline" | "unsupported" | "loading" | "error" | "empty" | "ready";

export function pluginPageState(input: {
  connected: boolean;
  supported: boolean;
  loading: boolean;
  error: boolean;
  pluginCount: number;
}): PluginPageState {
  if (!input.connected) return "offline";
  if (!input.supported) return "unsupported";
  if (input.loading) return "loading";
  if (input.error) return "error";
  return input.pluginCount === 0 ? "empty" : "ready";
}

/** Whether a machine of `protocol` serves the plugin calls. */
export function speaksPlugins(protocol: number): boolean {
  return protocol >= PLUGINS_PROTOCOL;
}

/** Paseo's install form: send only a non-empty, trimmed source. */
export function installSource(text: string): string | null {
  const source = text.trim();
  return source === "" ? null : source;
}

/** A row action, Paseo's `PluginRowAction`. */
export type PluginAction = "reload" | "enable" | "disable" | "remove";

/** The change in flight, so the controls wait (Paseo's pending mutation). */
export type PluginPending =
  { action: "install" | "switch" } | { action: PluginAction; plugin: string };

/** What a change said, under the install field (Paseo's feedback). */
export interface PluginFeedback {
  kind: "success" | "error";
  message: string;
}

/** A failed outcome's words. A machine's refusal is its own words alone,
 * as Paseo shows its daemon's error (`errorMessage(error)`): they already
 * say what happened and what to change. Anything else: what happened,
 * then what helps. */
export function problemWords(outcome: Outcome<unknown>): string {
  if (outcome.ok) return "";
  const { problem } = outcome;
  const refused =
    problem.from === "call" &&
    problem.failure.kind === "remote" &&
    problem.message.trim() !== "";
  return refused
    ? problem.message
    : `${problem.message} ${nextStep(problem)}`.trim();
}

/** What a status is called, and its badge tone (Paseo: running success,
 * failed error, otherwise muted). */
export function statusLook(status: PluginStatus): {
  word: string;
  tone: "ok" | "danger" | "neutral";
} {
  switch (status) {
    case "running":
      return { word: "running", tone: "ok" };
    case "failed":
      return { word: "failed", tone: "danger" };
    case "disabled":
      return { word: "disabled", tone: "neutral" };
  }
}

/** Paseo's feedback after a change (`settings.plugins.feedback.*`). */
export function changedWords(
  action: PluginAction | "install",
  id: string,
): string {
  switch (action) {
    case "install":
      return `Installed ${id}`;
    case "reload":
      return `Reloaded ${id}`;
    case "enable":
      return `Enabled ${id}`;
    case "disable":
      return `Disabled ${id}`;
    case "remove":
      return `Removed ${id}`;
  }
}

/** What Paseo shows under a row: the description, then the source. */
export function pluginSourceLine(plugin: PluginListItem): string | null {
  return plugin.path === undefined ? null : `Directory: ${plugin.path}`;
}

/** A log entry's head line: its time and stream, as Paseo's sheet. The
 * node keeps nanoseconds; Paseo's times are milliseconds
 * (`toISOString()`), so the head shows three digits. */
export function logHead(at: string, stream: "host" | "stderr"): string {
  const time = at.replace(/(\.\d{3})\d+Z$/, "$1Z");
  return `${time} ${stream === "host" ? "vessel" : "stderr"}`;
}

/** `node`'s plugins. */
export function listPlugins(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<readonly PluginListItem[]>> {
  return attempt(async () => {
    const list = await fleet.bridge.call(node, "plugin.list", {});
    return list.plugins;
  });
}

/** Whether plugins are on for `node` (absent means off). */
export function readSwitch(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<boolean>> {
  return attempt(async () => {
    const settings = await fleet.bridge.call(node, "node.settings", {});
    return settings.plugins_enabled === true;
  });
}

/** Turn plugins on or off for `node`; answers what is in force. */
export function setSwitch(
  fleet: Fleet,
  node: NodeKey,
  on: boolean,
): Promise<Outcome<boolean>> {
  return attempt(async () => {
    const settings = await fleet.bridge.call(node, "node.set_settings", {
      plugins_enabled: on,
    });
    return settings.plugins_enabled === true;
  });
}

/** Install from `source` (a directory on `node`). */
export function installPlugin(
  fleet: Fleet,
  node: NodeKey,
  source: string,
): Promise<Outcome<PluginListItem>> {
  return attempt(() => fleet.bridge.call(node, "plugin.install", { source }));
}

/** Run a row action on `node`'s installation `plugin`. */
export function pluginAction(
  fleet: Fleet,
  node: NodeKey,
  action: PluginAction,
  plugin: PluginListItem["id"],
): Promise<Outcome<null>> {
  return attempt(async () => {
    switch (action) {
      case "reload":
        await fleet.bridge.call(node, "plugin.reload", { plugin });
        return null;
      case "enable":
        await fleet.bridge.call(node, "plugin.enable", { plugin });
        return null;
      case "disable":
        await fleet.bridge.call(node, "plugin.disable", { plugin });
        return null;
      case "remove":
        await fleet.bridge.call(node, "plugin.remove", { plugin });
        return null;
    }
  });
}

/** `plugin`'s retained log tail on `node`. */
export function pluginLogs(
  fleet: Fleet,
  node: NodeKey,
  plugin: PluginListItem["id"],
): Promise<Outcome<PluginLogs>> {
  return attempt(() => fleet.bridge.call(node, "plugin.logs", { plugin }));
}

/** Wait for `node`'s plugin generation to move past `after` (a long poll,
 * standing for Paseo's `plugin_catalog_changed` push). */
export function watchPlugins(
  fleet: Fleet,
  node: NodeKey,
  after: PluginGeneration,
): Promise<Outcome<PluginGeneration>> {
  return attempt(() => fleet.bridge.call(node, "plugin.watch", { after }));
}
