// The shortcuts the shell answers (DESIGN.md §7.1): Settings, the "go to"
// places, the previous, next and numbered session in the sidebar's order,
// and the two panels. Registered once, by the shell.

import { useNavigate } from "@tanstack/react-router";

import { useFleet } from "../../context";
import { prefer, prefs } from "../../data/prefs";
import {
  sidebarOrder,
  stepSession,
  type SidebarSession,
} from "../../data/sidebar";
import { useWhere } from "../../data/where";
import { sidebarWorkspaces } from "../../data/workspace-labels/groups";
import { labelsOf } from "../../data/workspace-labels/hub";
import { useShortcut } from "../shortcuts/handlers";

export function useShellShortcuts(): void {
  const fleet = useFleet();
  const navigate = useNavigate();
  const where = useWhere();
  const [place, node, id] = where.segments;
  const current =
    place === "sessions" && node !== undefined && id !== undefined
      ? { node, id }
      : null;
  const go = (to: string) => {
    void navigate({ to });
    return true;
  };
  const open = (session: SidebarSession | null | undefined) => {
    if (session === null || session === undefined) {
      return false;
    }
    void navigate({
      to: "/sessions/$node/$id",
      params: { node: session.node, id: session.id },
    });
    return true;
  };
  const order = () => {
    const view = prefs.get().values.sidebar;
    // Grouped by workspace, the keys walk the workspace rows as drawn
    // (their membership needs the registry, which data/sidebar.ts does
    // not hold).
    return view.group === "workspace"
      ? sidebarWorkspaces(
          fleet.store.get(),
          labelsOf(fleet).store.get(),
          view,
          view.labels,
        ).flatMap((row) => row.sessions)
      : sidebarOrder(fleet.store.get(), view);
  };
  useShortcut("settings", () => go("/settings"));
  useShortcut("go-board", () => go("/sessions"));
  useShortcut("go-machines", () => go("/settings/machines"));
  useShortcut("go-sync", () => go("/settings/sync"));
  useShortcut("go-devices", () => go("/settings/devices"));
  useShortcut("next-session", () => open(stepSession(order(), current, 1)));
  useShortcut("previous-session", () =>
    open(stepSession(order(), current, -1)),
  );
  useShortcut("jump-session", (index) =>
    index === null ? false : open(order()[index - 1]),
  );
  useShortcut("toggle-sidebar", () => {
    const shown = prefs.get().values.layout.sidebar === "shown";
    void prefer(fleet, {
      set: "sidebar_panel",
      value: shown ? "hidden" : "shown",
    });
    return true;
  });
}
