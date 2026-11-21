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

/** The words of plugin surfaces, Paseo's. */
export const SURFACE_WORDS = {
  close: "Close plugin",
  offline: "This plugin's machine is offline.",
  unavailable: "This plugin surface is unavailable.",
  settingsOffline: "Connect to this machine to open plugin settings.",
  settingsUpdate: "Update this machine to use plugin settings.",
  settingsUnavailable: "This plugin settings screen is unavailable.",
  backToPlugins: "Back to plugins",
  reset: "Reset to defaults",
  reload: "Reload",
  retry: "Retry",
  loading: "Loading settings…",
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
  /** Unique across machines and plugins. */
  key: string;
  title: string;
  target: SurfaceTarget;
}

/** Every machine's `placement` rows, in machine, plugin and declaration
 * order; a row whose screen is not declared is left out. */
export function sidebarItems(
  catalogs: ReadonlyMap<NodeKey, MachineCatalog>,
  placement: SidebarPlacement,
): SidebarItem[] {
  const items: SidebarItem[] = [];
  for (const [node, { catalog }] of catalogs) {
    for (const installation of catalog.catalog.installations) {
      const plugin = installation.address.plugin;
      for (const item of installation.document.contributions) {
        const content = item.content;
        if (content.kind !== "sidebar" || content.placement !== placement)
          continue;
        items.push({
          key: `${node}:${plugin}:${placement}:${item.id}`,
          title: item.title,
          target: { kind: "screen", node, plugin, screen: content.screen },
        });
      }
    }
  }
  return items;
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
        order.set(
          `plugin:${node}:${installation.address.plugin}:${item.id}`,
          order.size,
        );
  return order;
}
