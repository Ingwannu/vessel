// A machine's plugin catalog on the page (ADR 0115, catalog and invoke):
// what each connected machine's running plugins declare, and the calls that
// run their methods. Paseo's client registry (`plugins/registry.ts`,
// `plugins/catalog-sync.tsx`) and its contribution views
// (`plugins/command-center/contributions.ts`,
// `plugins/client-slash-commands/`, `plugins/themes/`).
//
// [Decision Log]
// - Purpose: plugin Command Center items, slash commands and themes need
//   one place that holds each machine's declarations and decides which of
//   them a screen may show.
// - Constraint: the node answers declarations (ADR 0079), never code: a
//   command is a method call with declared input, a theme eight colours.
// - Choice: a store of the last catalog per machine (offline keeps it, a
//   machine taken out of the book drops it, no fallback to another
//   machine's plugins, as Paseo's registry), and pure functions over it that
//   the palette, the composer and Appearance read.
// - Invoke-bound items show only where the catalog says the caller may
//   invoke (`may_invoke`, coordinator ruling 2026-11-20). An `open_screen`
//   action waits for screens (P3b) and is left out until then.

import type { NodeKey } from "../wire/bridge";
import {
  type ClientInstallation,
  type CommandContext,
  type InvokeContext,
  PLUGIN_CATALOG_PROTOCOL,
  type PluginCatalog,
  type ProtocolVersion,
  type RawJson,
  type SessionId,
  type SessionWorkspace,
} from "../wire/proto";
import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";
import { Store } from "./store";

/** Whether a machine of `protocol` serves the catalog and method calls. */
export function speaksCatalog(protocol: ProtocolVersion): boolean {
  return protocol >= PLUGIN_CATALOG_PROTOCOL;
}

/** One machine's catalog as last read. */
export interface MachineCatalog {
  readonly catalog: PluginCatalog;
}

/** Every machine's last catalog, by book key. */
export const pluginCatalogs = new Store<ReadonlyMap<NodeKey, MachineCatalog>>(
  new Map(),
);

/** Keep `catalog` as `node`'s. */
export function keepCatalog(node: NodeKey, catalog: PluginCatalog): void {
  pluginCatalogs.update((all) => new Map(all).set(node, { catalog }));
}

/** Forget the machines not in `keys` (taken out of the book). */
export function keepOnly(keys: ReadonlySet<NodeKey>): void {
  pluginCatalogs.update((all) => {
    const kept = [...all].filter(([key]) => keys.has(key));
    return kept.length === all.size ? all : new Map(kept);
  });
}

/** `node`'s running plugins' declarations. */
export function readCatalog(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<PluginCatalog>> {
  return attempt(() => fleet.bridge.call(node, "plugin.catalog", {}));
}

/** Where an action is taken: no session, or the open session. */
export type PluginPlace =
  | { kind: "global" }
  | {
      kind: "session";
      node: NodeKey;
      agent: SessionId;
      workspace: SessionWorkspace | null;
    };

/** The invoke context a declared context needs at `place`, or `null` when
 * it is not available there (Paseo: a workspace command needs a
 * workspace, an agent command an agent). */
export function contextAt(
  wanted: CommandContext,
  place: PluginPlace,
  node: NodeKey,
): InvokeContext | null {
  if (wanted === "global") return { kind: "global" };
  if (place.kind !== "session" || place.node !== node) return null;
  const workspace = place.workspace;
  if (workspace === null) return null;
  const worktree =
    workspace.worktree === undefined || workspace.worktree === null
      ? {}
      : { worktree: workspace.worktree };
  return wanted === "workspace"
    ? { kind: "workspace", project: workspace.project, ...worktree }
    : {
        kind: "agent",
        project: workspace.project,
        ...worktree,
        agent: place.agent,
      };
}

/** A plugin method call as a screen captures it (ADR 0079: the run and
 * the context ride along, checked again at the node). */
export interface PluginCall {
  node: NodeKey;
  installation: ClientInstallation;
  method: string;
  input: RawJson;
  context: InvokeContext;
}

/** Run `call`; its output, or why not. */
export function invokePlugin(
  fleet: Fleet,
  call: PluginCall,
): Promise<Outcome<RawJson>> {
  return attempt(async () => {
    const answer = await fleet.bridge.call(call.node, "plugin.invoke", {
      plugin: call.installation.address.plugin,
      generation: call.installation.address.generation,
      method: call.method,
      input: call.input,
      context: call.context,
    });
    return answer.output;
  });
}

/** A Command Center item a plugin declared, runnable at a place. */
export interface PluginCommand {
  /** Unique across machines and plugins. */
  id: string;
  title: string;
  keywords: readonly string[];
  /** The plugin's id: the item's section, as Paseo's `sectionTitle`. */
  plugin: string;
  node: NodeKey;
  call: PluginCall;
}

/** Every machine's runnable Command Center items at `place`, in machine,
 * plugin and declaration order (Paseo's rank). */
export function pluginCommands(
  catalogs: ReadonlyMap<NodeKey, MachineCatalog>,
  place: PluginPlace,
): PluginCommand[] {
  const commands: PluginCommand[] = [];
  for (const [node, { catalog }] of catalogs) {
    if (!catalog.may_invoke) continue;
    for (const installation of catalog.catalog.installations) {
      for (const item of installation.document.contributions) {
        const content = item.content;
        if (content.kind !== "command") continue;
        if (content.action.kind !== "invoke") continue;
        const context = contextAt(content.context, place, node);
        if (context === null) continue;
        const plugin = installation.address.plugin;
        commands.push({
          id: `plugin:${node}:${plugin}:${item.id}`,
          title: item.title,
          keywords: content.keywords,
          plugin,
          node,
          call: {
            node,
            installation,
            method: content.action.method,
            input: content.action.input,
            context,
          },
        });
      }
    }
  }
  return commands;
}

/** A slash command a plugin declared for the open session's machine. */
export interface PluginSlashCommand {
  name: string;
  description: string;
  argumentHint: string;
  plugin: string;
  /** Its call; the typed arguments become its input. */
  call: (args: string) => PluginCall;
}

/** The slash commands `node`'s plugins offer at `place` (Paseo's
 * `usePluginClientSlashCommands`: the open session's machine only). */
export function pluginSlashCommands(
  catalogs: ReadonlyMap<NodeKey, MachineCatalog>,
  node: NodeKey,
  place: PluginPlace,
): PluginSlashCommand[] {
  const held = catalogs.get(node);
  if (held?.catalog.may_invoke !== true) return [];
  const commands: PluginSlashCommand[] = [];
  for (const installation of held.catalog.catalog.installations) {
    for (const item of installation.document.contributions) {
      const content = item.content;
      if (content.kind !== "slash_command") continue;
      const context = contextAt(content.context, place, node);
      if (context === null) continue;
      commands.push({
        name: content.name,
        description: content.description,
        argumentHint: content.argument_hint ?? "",
        plugin: installation.address.plugin,
        call: (args) => ({
          node,
          installation,
          method: content.method,
          input: { arguments: args },
          context,
        }),
      });
    }
  }
  return commands;
}

/** Paseo's `resolvePluginClientSlashCommand`: `/name args` naming one of
 * `commands`, with no attachments; `null` otherwise. */
export function resolveSlashCommand<Command extends { name: string }>(
  text: string,
  hasAttachments: boolean,
  commands: readonly Command[],
): { command: Command; args: string } | null {
  if (hasAttachments) return null;
  const match = /^\/(\S+)(?:\s+([\s\S]*))?$/.exec(text.trim());
  if (match === null) return null;
  const command = commands.find((candidate) => candidate.name === match[1]);
  return command === undefined
    ? null
    : { command, args: (match[2] ?? "").trim() };
}
