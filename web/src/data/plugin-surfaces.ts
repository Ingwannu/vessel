// A machine's plugin surfaces on the page (ADR 0115, "Surfaces"): the
// screens, settings screens and sidebar items its running plugins declare,
// and the Command Center items that open them. Paseo's
// `plugins/surface-screen.tsx`, `plugins/sidebar-items/`,
// `plugins/settings/index.tsx` and `plugins/actions.ts`.
//
// [Decision Log]
// - Purpose: the sidebar, the screen route, Settings › Plugins and the
//   palette must agree on which surface a (machine, plugin, id) names and
//   whether this device may see it.
// - Constraint: the node sends declarations, never components (ADR 0079),
//   so a "screen" is a declared `Surface` this page draws with its kit.
// - Choice: pure lookups over the catalog store (data/plugin-catalog.ts),
//   tested on their own. A screen is drawn for every device that reads the
//   catalog; a settings screen only where `may_invoke`, because reading
//   settings needs admin (coordinator ruling on P3b, Q1).
// - Paseo's words, with "host" read as "machine" as Settings › Plugins
//   reads it.

import type { NodeKey } from "../wire/bridge";
import type {
  ClientInstallation,
  Contribution,
  SidebarPlacement,
  Surface,
} from "../wire/proto";
import {
  contextAt,
  type MachineCatalog,
  type PluginPlace,
} from "./plugin-catalog";
import { Store } from "./store";

/** The words of plugin surfaces, Paseo's. */
export const SURFACE_WORDS = {
  close: "Close plugin",
  // Paseo's "Plugin host is offline.", with "host" read as "machine".
  offline: "Plugin machine is offline.",
  unavailable: "This plugin surface is unavailable.",
  settingsOffline: "Connect to this machine to open plugin settings.",
  settingsUpdate: "Update this machine to use plugin settings.",
  settingsUnavailable: "This plugin settings screen is unavailable.",
  backToPlugins: "Back to plugins",
  // Paseo's detail title when the screen is gone: `<plugin> · Settings`.
  settingsTitle: "Settings",
  reset: "Reset to defaults",
  reload: "Reload",
  retry: "Retry",
  loading: "Loading settings…",
  conflict: "Settings changed on another client. Reload before saving again.",
} as const;

/** The muted line on a surface drawn for a device without admin. */
export function readOnlyLine(machine: string): string {
  return `Changing this needs admin access to ${machine}.`;
}

/** Which kind of surface an address names. */
export type SurfaceKind = "screen" | "settings_screen";

/** A declared surface found in a machine's catalog. */
export interface FoundSurface {
  node: NodeKey;
  installation: ClientInstallation;
  item: Contribution;
  surface: Surface;
  mayInvoke: boolean;
}

/** The `kind` surface `id` of `plugin` in `held`, `node`'s last catalog,
 * or `null` when the machine has no such running plugin or declaration. */
export function findSurface(
  held: MachineCatalog | undefined,
  node: NodeKey,
  plugin: string,
  kind: SurfaceKind,
  id: string,
): FoundSurface | null {
  if (held === undefined) return null;
  const installation = held.catalog.catalog.installations.find(
    (entry) => entry.address.plugin === plugin,
  );
  if (installation === undefined) return null;
  for (const item of installation.document.contributions) {
    const content = item.content;
    if (item.id !== id || content.kind !== kind) continue;
    if (kind === "settings_screen" && !held.catalog.may_invoke) return null;
    return {
      node,
      installation,
      item,
      surface: content.surface,
      mayInvoke: held.catalog.may_invoke,
    };
  }
  return null;
}

/** A plugin's settings screens, for its row's menu; none without admin. */
export function settingsScreens(
  held: MachineCatalog | undefined,
  plugin: string,
): readonly Contribution[] {
  if (held?.catalog.may_invoke !== true) return [];
  const installation = held.catalog.catalog.installations.find(
    (entry) => entry.address.plugin === plugin,
  );
  return (installation?.document.contributions ?? []).filter(
    (item) => item.content.kind === "settings_screen",
  );
}

/** Where a surface is opened: the route and its search. */
export type SurfaceTarget =
  | {
      kind: "screen";
      node: NodeKey;
      plugin: string;
      screen: string;
    }
  | {
      kind: "settings";
      node: NodeKey;
      plugin: string;
      screen: string;
    };

/** A sidebar row a plugin declared, opening one of its screens. */
export interface SidebarItem {
  /** One per plugin and item, across machines. */
  key: string;
  title: string;
  target: SurfaceTarget;
}

/** How a row declared on several machines picks the one it opens. */
export interface SidebarChoice {
  /** Whether a machine is connected; an away machine's rows are left out. */
  online: (node: NodeKey) => boolean;
  /** The plugin screen open now, if any. */
  current: { node: NodeKey; plugin: string; screen: string } | null;
  /** The machine each plugin screen was last shown from, by
   * `screenKey`. */
  remembered: ReadonlyMap<string, NodeKey>;
}

/** The key of a plugin's screen for `SidebarChoice.remembered`. */
export function screenKey(plugin: string, screen: string): string {
  return JSON.stringify([plugin, screen]);
}

/** The machines last shown for each plugin screen, by `screenKey`. */
export const rememberedScreens = new Store<ReadonlyMap<string, NodeKey>>(
  new Map(),
);

/** Remember that `plugin`'s `screen` was shown from `node`. */
export function rememberScreen(
  node: NodeKey,
  plugin: string,
  screen: string,
): void {
  const key = screenKey(plugin, screen);
  rememberedScreens.update((all) =>
    all.get(key) === node ? all : new Map(all).set(key, node),
  );
}

/** The connected machines' `placement` rows, one per plugin and item (Paseo
 * merges an item across hosts, sidebar-groups.ts:33-37, 97-110): it opens
 * the machine whose screen is open now, else the one last shown, else the
 * first in machine order (reference.md:727-729; node review R4). */
export function sidebarItems(
  catalogs: ReadonlyMap<NodeKey, MachineCatalog>,
  placement: SidebarPlacement,
  choice: SidebarChoice,
): SidebarItem[] {
  const merged = new Map<
    string,
    { title: string; plugin: string; screen: string; nodes: NodeKey[] }
  >();
  for (const [node, { catalog }] of catalogs) {
    if (!choice.online(node)) continue;
    for (const installation of catalog.catalog.installations) {
      const plugin = installation.address.plugin;
      for (const item of installation.document.contributions) {
        const content = item.content;
        if (content.kind !== "sidebar" || content.placement !== placement)
          continue;
        const key = JSON.stringify([plugin, placement, item.id]);
        const held = merged.get(key);
        if (held === undefined)
          merged.set(key, {
            title: item.title,
            plugin,
            screen: content.screen,
            nodes: [node],
          });
        else held.nodes.push(node);
      }
    }
  }
  return [...merged].flatMap(([key, item]) => {
    const now = choice.current;
    const last = choice.remembered.get(screenKey(item.plugin, item.screen));
    const node =
      now !== null &&
      now.plugin === item.plugin &&
      now.screen === item.screen &&
      item.nodes.includes(now.node)
        ? now.node
        : last !== undefined && item.nodes.includes(last)
          ? last
          : item.nodes[0];
    return node === undefined
      ? []
      : [
          {
            key,
            title: item.title,
            target: {
              kind: "screen" as const,
              node,
              plugin: item.plugin,
              screen: item.screen,
            },
          },
        ];
  });
}

/** A Command Center item that opens a plugin's screen or settings screen. */
export interface SurfaceCommand {
  id: string;
  title: string;
  keywords: readonly string[];
  plugin: string;
  target: SurfaceTarget;
}

/** Every machine's opening items at `place`: `open_screen` for every device
 * that reads the catalog, `open_settings` only where `may_invoke`. */
export function surfaceCommands(
  catalogs: ReadonlyMap<NodeKey, MachineCatalog>,
  place: PluginPlace,
): SurfaceCommand[] {
  const commands: SurfaceCommand[] = [];
  for (const [node, { catalog }] of catalogs) {
    for (const installation of catalog.catalog.installations) {
      const plugin = installation.address.plugin;
      for (const item of installation.document.contributions) {
        const content = item.content;
        if (content.kind !== "command") continue;
        const action = content.action;
        if (action.kind === "invoke") continue;
        if (action.kind === "open_settings" && !catalog.may_invoke) continue;
        if (contextAt(content.context, place, node) === null) continue;
        commands.push({
          id: `plugin:${node}:${plugin}:${item.id}`,
          title: item.title,
          keywords: content.keywords,
          plugin,
          target: {
            kind: action.kind === "open_screen" ? "screen" : "settings",
            node,
            plugin,
            screen: action.screen,
          },
        });
      }
    }
  }
  return commands;
}

/** Each declared contribution's place in machine, plugin and declaration
 * order, by its palette id: the palette lists methods and opening items
 * mixed in that order, as Paseo's Command Center ranks them. */
export function declarationOrder(
  catalogs: ReadonlyMap<NodeKey, MachineCatalog>,
): ReadonlyMap<string, number> {
  const order = new Map<string, number>();
  for (const [node, { catalog }] of catalogs)
    for (const installation of catalog.catalog.installations)
      for (const item of installation.document.contributions)
        if (item.content.kind === "command")
          order.set(
            `plugin:${node}:${installation.address.plugin}:${item.id}`,
            order.size,
          );
  return order;
}
