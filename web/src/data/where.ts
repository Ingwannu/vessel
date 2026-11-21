// Where the page is: path segments and the query parameters the pages
// read (`node`, `machine`, `agent`).
//
// Read from the router's location rather than its typed params hooks: with
// code-based routes whose components are typed through the same router,
// TanStack Router's loose (`strict: false`) params and search come out as
// `any`, which the lint rules (no `any`) refuse. The pathname and the
// query string are plain strings either way.
//
// A machine named in the query is a `NodeKey` only when it has a key's
// shape (`bridge/ids.ts`, the page's one place for brands); anything else
// is as if none were given.

import { useRouterState } from "@tanstack/react-router";

import { nodeKeyOf } from "../bridge/ids";
import type { NodeKey } from "../wire/bridge";

export interface Where {
  /** The path's segments, decoded (`/sessions/box/3f2a` → sessions, box, 3f2a). */
  segments: readonly string[];
  /** `?node=`, when given. */
  node: NodeKey | undefined;
  /** `?machine=` (the new-session page's and Settings'), when given. */
  machine: NodeKey | undefined;
  /** `?agent=` (Settings' Agents: the open card), when given. */
  agent: string | undefined;
  /** `?plugin=` and `?screen=` (Settings › Plugins: an open settings
   * screen, ADR 0115 "Surfaces"), when given. */
  plugin: string | undefined;
  screen: string | undefined;
}

/** A machine's key from a query value, when it is one. */
function machineValue(raw: string | null): NodeKey | undefined {
  const text = queryValue(raw);
  return text === undefined ? undefined : (nodeKeyOf(text) ?? undefined);
}

/** A query value as the router writes it: plain text, or JSON text. */
function queryValue(raw: string | null): string | undefined {
  if (raw === null) {
    return undefined;
  }
  try {
    const parsed: unknown = JSON.parse(raw);
    return typeof parsed === "string" ? parsed : raw;
  } catch {
    return raw;
  }
}

export function useWhere(): Where {
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const query = useRouterState({ select: (state) => state.location.searchStr });
  const segments = pathname
    .split("/")
    .filter((segment) => segment !== "")
    .map((segment) => decodeURIComponent(segment));
  const params = new URLSearchParams(query);
  return {
    segments,
    node: machineValue(params.get("node")),
    machine: machineValue(params.get("machine")),
    agent: queryValue(params.get("agent")),
    plugin: queryValue(params.get("plugin")),
    screen: queryValue(params.get("screen")),
  };
}

/** The session the page shows (`/sessions/$node/$id`), if one. */
export function shownSession(
  where: Where,
): { node: NodeKey; id: string } | null {
  const [first, node, id] = where.segments;
  const key = node === undefined ? null : nodeKeyOf(node);
  return first === "sessions" && key !== null && id !== undefined
    ? { node: key, id }
    : null;
}

/** The plugin screen the page shows (`/plugin/$node/$plugin/$screen`,
 * ADR 0115 "Surfaces"), if one. */
export function shownPluginScreen(
  where: Where,
): { node: NodeKey; plugin: string; screen: string } | null {
  const [first, shown, plugin, screen] = where.segments;
  const node = nodeKeyOf(shown ?? "");
  return first === "plugin" &&
    node !== null &&
    plugin !== undefined &&
    screen !== undefined
    ? { node, plugin, screen }
    : null;
}
