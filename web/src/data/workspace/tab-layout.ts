// The page's ordered workspace targets. IDs come from their target so
// opening a reference twice focuses its existing tab and editor.

import { isSessionId, isTerminalId } from "../../bridge/ids";
import type { NodeKey } from "../../wire/bridge";
import type {
  RelPath,
  SessionId,
  TerminalId,
  WorkspaceRoot,
} from "../../wire/proto";
import { rootKey } from "./root";

export type WorkspaceTab =
  | { kind: "agent"; id: SessionId }
  | { kind: "terminal"; id: TerminalId }
  | { kind: "file"; path: RelPath }
  | { kind: "files" }
  | { kind: "changes" }
  | { kind: "diff" }
  | { kind: "draft" }
  /** The anchor session's pull request (TRACKER 430, Paseo's pane). */
  | { kind: "pull" };

export interface TabLayout {
  tabs: readonly WorkspaceTab[];
  active: string | null;
  route: SessionId | null;
  hidden: readonly SessionId[];
}

export type CloseScope = "before" | "after" | "others";

const KEY_PREFIX = "vessel.workspace-tabs.v1.";

export const EMPTY_LAYOUT: TabLayout = {
  tabs: [],
  active: null,
  route: null,
  hidden: [],
};

export function workspaceTabScope(
  node: NodeKey,
  root: WorkspaceRoot | null,
  session: SessionId,
): string {
  return (
    KEY_PREFIX +
    node +
    ":" +
    (root === null ? `session:${session}` : rootKey(root))
  );
}

export function tabKey(tab: WorkspaceTab): string {
  switch (tab.kind) {
    case "agent":
    case "terminal":
      return `${tab.kind}:${tab.id}`;
    case "file":
      return `file:${tab.path}`;
    case "files":
    case "changes":
    case "diff":
    case "draft":
    case "pull":
      return tab.kind;
  }
}

export function showTab(layout: TabLayout, tab: WorkspaceTab): TabLayout {
  const active = tabKey(tab);
  const exists = layout.tabs.some((item) => tabKey(item) === active);
  const hidden =
    tab.kind === "agent" && layout.hidden.includes(tab.id)
      ? layout.hidden.filter((id) => id !== tab.id)
      : layout.hidden;
  return exists && layout.active === active && hidden === layout.hidden
    ? layout
    : {
        ...layout,
        tabs: exists ? layout.tabs : [...layout.tabs, tab],
        active,
        hidden,
      };
}

/** Returning to the same URL restores passive selection. A new agent
 * URL always reveals its agent, including deep links into the archive. */
export function enterRoute(layout: TabLayout, route: SessionId): TabLayout {
  return layout.route === route
    ? layout
    : { ...showTab(layout, { kind: "agent", id: route }), route };
}

/** Append discoveries without displacing the active tab. Failed reads
 * and incomplete lists never remove previously open targets. */
export function addDiscovered(
  layout: TabLayout,
  discovered: readonly WorkspaceTab[],
): TabLayout {
  const keys = new Set(layout.tabs.map(tabKey));
  const hidden = new Set(layout.hidden);
  const added = discovered.filter((tab) => {
    const key = tabKey(tab);
    if (keys.has(key) || (tab.kind === "agent" && hidden.has(tab.id))) {
      return false;
    }
    keys.add(key);
    return true;
  });
  return added.length === 0
    ? layout
    : { ...layout, tabs: [...layout.tabs, ...added] };
}

export function removeTab(layout: TabLayout, key: string): TabLayout {
  const index = layout.tabs.findIndex((tab) => tabKey(tab) === key);
  const removed = layout.tabs[index];
  if (removed === undefined) {
    return layout;
  }
  const tabs = layout.tabs.filter((tab) => tabKey(tab) !== key);
  const next = tabs[index] ?? tabs[index - 1];
  return {
    ...layout,
    tabs,
    active:
      layout.active === key
        ? next === undefined
          ? null
          : tabKey(next)
        : layout.active,
    route:
      removed.kind === "agent" && removed.id === layout.route
        ? null
        : layout.route,
    hidden:
      removed.kind === "agent" ? [...layout.hidden, removed.id] : layout.hidden,
  };
}

/** Keep the draft's position and respect a tab switch while creating. */
export function settleDraft(layout: TabLayout, id: SessionId): TabLayout {
  const agent = { kind: "agent", id } as const;
  const next = tabKey(agent);
  if (!layout.tabs.some((tab) => tab.kind === "draft")) {
    return addDiscovered(layout, [agent]);
  }
  return {
    ...layout,
    tabs: layout.tabs
      .filter((tab) => tabKey(tab) !== next)
      .map((tab) => (tab.kind === "draft" ? agent : tab)),
    active: layout.active === "draft" ? next : layout.active,
  };
}

export function tabsToClose(
  tabs: readonly WorkspaceTab[],
  key: string,
  scope: CloseScope,
): readonly WorkspaceTab[] {
  const index = tabs.findIndex((tab) => tabKey(tab) === key);
  if (index < 0) {
    return [];
  }
  switch (scope) {
    case "before":
      return tabs.slice(0, index);
    case "after":
      return tabs.slice(index + 1);
    case "others":
      return tabs.filter((tab) => tabKey(tab) !== key);
  }
}

export function moveTab(
  layout: TabLayout,
  source: string,
  target: string,
  side: "before" | "after",
): TabLayout {
  const item = layout.tabs.find((tab) => tabKey(tab) === source);
  if (item === undefined || source === target) {
    return layout;
  }
  const tabs = layout.tabs.filter((tab) => tabKey(tab) !== source);
  const index = tabs.findIndex((tab) => tabKey(tab) === target);
  if (index < 0) {
    return layout;
  }
  tabs.splice(index + (side === "after" ? 1 : 0), 0, item);
  return { ...layout, tabs };
}

function readTab(value: unknown): WorkspaceTab | null {
  if (typeof value !== "object" || value === null || !("kind" in value)) {
    return null;
  }
  switch (value.kind) {
    case "agent":
      return "id" in value &&
        typeof value.id === "string" &&
        isSessionId(value.id)
        ? { kind: "agent", id: value.id }
        : null;
    case "terminal":
      return "id" in value &&
        typeof value.id === "string" &&
        isTerminalId(value.id)
        ? { kind: "terminal", id: value.id }
        : null;
    case "file":
      return "path" in value &&
        typeof value.path === "string" &&
        value.path !== ""
        ? { kind: "file", path: value.path }
        : null;
    case "files":
    case "changes":
    case "diff":
    case "draft":
    case "pull":
      return { kind: value.kind };
    default:
      return null;
  }
}

function ids(value: unknown): SessionId[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter(
    (id): id is SessionId => typeof id === "string" && isSessionId(id),
  );
}

export function loadTabLayout(storage: Storage, scope: string): TabLayout {
  try {
    const value: unknown = JSON.parse(storage.getItem(scope) ?? "null");
    if (
      typeof value !== "object" ||
      value === null ||
      !("tabs" in value) ||
      !Array.isArray(value.tabs)
    ) {
      return EMPTY_LAYOUT;
    }
    const tabs = addDiscovered(
      EMPTY_LAYOUT,
      value.tabs
        .map(readTab)
        .filter((tab): tab is WorkspaceTab => tab !== null),
    ).tabs;
    const active =
      "active" in value && typeof value.active === "string"
        ? value.active
        : null;
    return {
      tabs,
      active: tabs.some((tab) => tabKey(tab) === active) ? active : null,
      route:
        "route" in value &&
        typeof value.route === "string" &&
        isSessionId(value.route)
          ? value.route
          : null,
      hidden: "hidden" in value ? ids(value.hidden) : [],
    };
  } catch {
    return EMPTY_LAYOUT;
  }
}

export function saveTabLayout(
  storage: Storage,
  scope: string,
  layout: TabLayout,
): void {
  try {
    storage.setItem(scope, JSON.stringify(layout));
  } catch {
    // The open workspace still works when private browsing refuses storage.
  }
}
