// The sidebar's search row and the command palette it opens (DESIGN.md
// §2.2): its shortcut (`⌘K` or Ctrl+K anywhere, `/` when not typing, by
// default; data/shortcuts/registry.ts).
//
// The row is a button drawn as the reference's search pill, not a text
// field: typing happens in the palette, so the sidebar never holds a
// second, smaller search box. What the palette lists is
// `data/commands.ts`; this only maps its targets to the router.
//
// The palette and its shortcut are the shell's (`PaletteHost`), mounted
// once whether the sidebar is shown, hidden or a sheet: the shortcut was
// the sidebar's, and hiding the sidebar (milestone 11) left Mod+K to the
// browser. Only the open palette reads the whole fleet.

import "./palette.css";

import { useNavigate, useRouterState } from "@tanstack/react-router";
import { useMemo } from "react";

import { openNewSession, useFleet, useFleetState } from "../../context";
import { openAddMachine } from "../../data/add-machine";
import type { FleetState } from "../../data/fleet";
import { labelManager } from "../../data/workspace-labels/hub";
import {
  type PaletteEntry,
  type PalettePlace,
  paletteEntries,
} from "../../data/commands";
import { selectedNode } from "../../data/nodes";
import { closePalette, openPalette, palette } from "../../data/palette";
import {
  invokePlugin,
  type PluginPlace,
  pluginCatalogs,
  pluginCommands,
} from "../../data/plugin-catalog";
import { problemWords } from "../../data/plugins";
import { declarationOrder, surfaceCommands } from "../../data/plugin-surfaces";
import { useStore } from "../../data/store";
import { chooseTheme } from "../../data/theme";
import {
  type Command,
  CommandPalette,
  Icon,
  type IconName,
  Kbd,
} from "../../ui/kit";
import { toast } from "../../ui/toasts";
import { useOpenSurface } from "../plugins/use-open-surface";
import { sectionIcon } from "../settings/section-icon";
import { useShortcut } from "../shortcuts/handlers";
import { useShortcutWords } from "../shortcuts/use-keys";

function iconOf(place: PalettePlace): IconName {
  if (typeof place === "object") {
    return sectionIcon(place.section);
  }
  switch (place) {
    case "board":
      return "board";
    case "schedules":
      return "calendar-clock";
    case "settings":
      return "settings";
    case "machine":
      return "machine";
    case "terminal":
      return "terminal";
    case "structured":
      return "message";
    case "new":
      return "plus";
    case "add-machine":
      return "machine";
    case "labels":
      return "tag";
    case "theme-system":
      return "monitor";
    case "theme-light":
      return "sun";
    case "theme-dark":
      return "moon";
  }
}

/** Where the palette was opened, for a plugin item's context: the open
 * session (`/sessions/<node>/<id>`), or no session. */
function placeOf(pathname: string, state: FleetState): PluginPlace {
  const match = /^\/sessions\/([^/]+)\/([^/]+)/.exec(pathname);
  if (match === null) return { kind: "global" };
  const named = decodeURIComponent(match[1] ?? "");
  const id = decodeURIComponent(match[2] ?? "");
  const node = (state.book ?? []).find((entry) => entry.key === named)?.key;
  if (node === undefined) return { kind: "global" };
  const session = (state.sessions[node] ?? []).find(
    (candidate) => candidate.id === id,
  );
  return session === undefined
    ? { kind: "global" }
    : {
        kind: "session",
        node,
        agent: session.id,
        workspace: session.workspace ?? null,
      };
}

/** The palette, while open: what it lists comes from the fleet as it is
 * now, then the machines' plugin items (Paseo's Command Center: a section
 * per plugin, after the app's own; an error is a toast, success is the
 * plugin's own). */
function Palette() {
  const navigate = useNavigate();
  const fleet = useFleet();
  const state = useFleetState((value) => value);
  const catalogs = useStore(pluginCatalogs, (value) => value);
  const pathname = useRouterState({
    select: (router) => router.location.pathname,
  });
  const openSurface = useOpenSurface();
  const plugins = useMemo(() => {
    const place = placeOf(pathname, state);
    const order = declarationOrder(catalogs);
    const opening = surfaceCommands(catalogs, place).map(
      (command): Command => ({
        id: command.id,
        label: command.title,
        ...(command.keywords.length === 0
          ? {}
          : { keywords: command.keywords.join(" ") }),
        icon: "blocks",
        group: command.plugin,
        run: () => {
          openSurface(command.target);
        },
      }),
    );
    const running = pluginCommands(catalogs, place).map((command): Command => ({
      id: command.id,
      label: command.title,
      ...(command.keywords.length === 0
        ? {}
        : { keywords: command.keywords.join(" ") }),
      icon: "blocks",
      group: command.plugin,
      run: () => {
        void invokePlugin(fleet, command.call).then((outcome) => {
          if (!outcome.ok) toast(problemWords(outcome), "error");
        });
      },
    }));
    return [...running, ...opening].sort(
      (a, b) => (order.get(a.id) ?? 0) - (order.get(b.id) ?? 0),
    );
  }, [catalogs, pathname, state, fleet, openSurface]);
  const commands = useMemo(
    () =>
      paletteEntries(
        state,
        selectedNode(state, undefined),
        navigator.language,
      ).map((entry: PaletteEntry): Command => ({
        id: entry.id,
        label: entry.label,
        ...(entry.detail === undefined ? {} : { detail: entry.detail }),
        ...(entry.keywords === undefined ? {} : { keywords: entry.keywords }),
        icon: iconOf(entry.place),
        group: entry.group,
        run: () => {
          const target = entry.target;
          switch (target.kind) {
            case "route":
              void navigate({ to: target.to });
              return;
            case "new-session":
              openNewSession(target.node);
              return;
            case "add-machine":
              openAddMachine();
              return;
            case "manage-labels":
              labelManager.set(true);
              return;
            case "theme":
              void chooseTheme(fleet, target.preference);
              return;
          }
        },
      })),
    [state, navigate, fleet],
  );
  const all = useMemo(() => [...commands, ...plugins], [commands, plugins]);
  return <CommandPalette commands={all} onClose={closePalette} />;
}

/** The palette and its shortcut, once, in the shell. */
export function PaletteHost() {
  const open = useStore(palette, (value) => value);
  useShortcut("palette", () => {
    openPalette();
    return true;
  });
  return open ? <Palette /> : null;
}

/** The sidebar's search row: it opens the palette. */
export function SidebarSearch() {
  const keys = useShortcutWords("palette");
  return (
    <div className="side-search-wrap">
      <button
        type="button"
        className="side-search"
        aria-haspopup="dialog"
        onClick={openPalette}
      >
        <Icon name="search" size={14} />
        <span className="label">Search</span>
        {keys === null ? null : <Kbd keys={[keys]} look="text" />}
      </button>
    </div>
  );
}
