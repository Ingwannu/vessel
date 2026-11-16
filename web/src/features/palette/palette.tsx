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

import { useNavigate } from "@tanstack/react-router";
import { useMemo } from "react";

import { openNewSession, useFleet, useFleetState } from "../../context";
import { openAddMachine } from "../../data/add-machine";
import { labelManager } from "../../data/workspace-labels/hub";
import {
  type PaletteEntry,
  type PalettePlace,
  paletteEntries,
} from "../../data/commands";
import { selectedNode } from "../../data/nodes";
import { closePalette, openPalette, palette } from "../../data/palette";
import { useStore } from "../../data/store";
import { chooseTheme } from "../../data/theme";
import {
  type Command,
  CommandPalette,
  Icon,
  type IconName,
  Kbd,
} from "../../ui/kit";
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

/** The palette, while open: what it lists comes from the fleet as it is
 * now. */
function Palette() {
  const navigate = useNavigate();
  const fleet = useFleet();
  const state = useFleetState((value) => value);
  const commands = useMemo(
    () =>
      paletteEntries(state, selectedNode(state, undefined)).map(
        (entry: PaletteEntry): Command => ({
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
        }),
      ),
    [state, navigate, fleet],
  );
  return <CommandPalette commands={commands} onClose={closePalette} />;
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
