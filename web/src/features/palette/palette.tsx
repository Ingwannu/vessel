// The sidebar's search row and the command palette it opens (DESIGN.md
// §2.2): `⌘K` (or Ctrl+K) anywhere, `/` when not typing.
//
// The row is a button drawn as the reference's search pill, not a text
// field: typing happens in the palette, so the sidebar never holds a
// second, smaller search box. What the palette lists is
// `data/commands.ts`; this only maps its targets to the router.

import "./palette.css";

import { useNavigate } from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";

import { openNewSession, useFleetState } from "../../context";
import { openAddMachine } from "../../data/add-machine";
import {
  type PaletteEntry,
  type PalettePlace,
  paletteEntries,
} from "../../data/commands";
import { selectedNode } from "../../data/nodes";
import { chooseTheme } from "../../data/theme";
import {
  type Command,
  CommandPalette,
  Icon,
  type IconName,
  Kbd,
} from "../../ui/kit";
import { typing } from "./shortcuts";

function iconOf(place: PalettePlace): IconName {
  switch (place) {
    case "fleet":
      return "fleet";
    case "sessions":
      return "sessions";
    case "sync":
      return "sync";
    case "devices":
      return "devices";
    case "settings":
      return "settings";
    case "node":
      return "home";
    case "terminal":
      return "terminal";
    case "structured":
      return "message";
    case "new":
      return "plus";
    case "add-machine":
      return "server";
    case "theme-system":
      return "monitor";
    case "theme-light":
      return "sun";
    case "theme-dark":
      return "moon";
  }
}

export function SidebarSearch() {
  const [open, setOpen] = useState(false);
  const navigate = useNavigate();
  const state = useFleetState((fleet) => fleet);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const chord = event.key === "k" && (event.metaKey || event.ctrlKey);
      if (chord || (event.key === "/" && !typing(event.target))) {
        event.preventDefault();
        setOpen(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, []);
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
              case "theme":
                chooseTheme(target.preference);
                return;
            }
          },
        }),
      ),
    [state, navigate],
  );
  return (
    <div className="side-search-wrap">
      <button
        type="button"
        className="side-search"
        aria-haspopup="dialog"
        onClick={() => {
          setOpen(true);
        }}
      >
        <Icon name="search" size={14} />
        <span className="label">Search</span>
        <Kbd keys={["⌘", "K"]} look="text" />
      </button>
      {open ? (
        <CommandPalette
          commands={commands}
          onClose={() => {
            setOpen(false);
          }}
        />
      ) : null}
    </div>
  );
}
