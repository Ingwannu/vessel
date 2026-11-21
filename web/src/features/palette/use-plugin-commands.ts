// Paseo registers plugin actions for the focused session's machine.
// Vessel also offers them on fleet routes, where the machine label says
// which installation each command runs against.

import { useMemo } from "react";

import { useFleet, useFleetState } from "../../context";
import { nodeLabel } from "../../data/nodes";
import {
  invokePlugin,
  type MachineCatalog,
  type PluginPlace,
  pluginCatalogs,
  pluginCommands,
} from "../../data/plugin-catalog";
import { declarationOrder, surfaceCommands } from "../../data/plugin-surfaces";
import { problemWords } from "../../data/plugins";
import { useStore } from "../../data/store";
import { shownSession, useWhere } from "../../data/where";
import type { Command } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import { useOpenSurface } from "../plugins/use-open-surface";

const NO_CATALOGS: ReadonlyMap<NodeKey, MachineCatalog> = new Map();

interface Item {
  id: string;
  plugin: string;
  title: string;
  keywords: readonly string[];
}

function command(item: Item, detail: string | null, run: () => void): Command {
  return {
    id: item.id,
    label: item.title,
    ...(item.keywords.length === 0
      ? {}
      : { keywords: item.keywords.join(" ") }),
    ...(detail === null ? {} : { detail }),
    icon: "blocks",
    group: item.plugin,
    run,
  };
}

/** Rebuild plugin commands only when their catalog, context or labels move. */
export function usePluginCommands(): Command[] {
  const fleet = useFleet();
  const openSurface = useOpenSurface();
  const shown = shownSession(useWhere());
  const focus = shown?.node ?? null;
  const session = useFleetState((state) =>
    shown === null
      ? null
      : (state.sessions[shown.node]?.find((row) => row.id === shown.id) ??
        null),
  );
  const agent = session?.id ?? null;
  const project = session?.workspace?.project ?? null;
  const worktree = session?.workspace?.worktree ?? null;
  const book = useFleetState((state) => (focus === null ? state.book : null));
  const hereLabel = useFleetState((state) => {
    if (focus !== null) return null;
    const here = state.book?.find((entry) => entry.key === "_here");
    return here === undefined
      ? null
      : nodeLabel(here, state.statuses[here.key]);
  });
  const all = useStore(pluginCatalogs, (value) =>
    focus === null ? value : NO_CATALOGS,
  );
  const held = useStore(pluginCatalogs, (value) =>
    focus === null ? null : (value.get(focus) ?? null),
  );
  return useMemo(() => {
    const catalogs =
      focus !== null && held !== null ? new Map([[focus, held]]) : all;
    const place: PluginPlace =
      focus === null || agent === null
        ? { kind: "global" }
        : {
            kind: "session",
            node: focus,
            agent,
            workspace: project === null ? null : { project, worktree },
          };
    const labels = new Map(book?.map((entry) => [entry.key, entry.label]));
    const detail = (node: NodeKey): string | null => {
      if (focus !== null) return null;
      return (node === "_here" ? hereLabel : labels.get(node)) ?? node;
    };
    const opening = surfaceCommands(catalogs, place).map((item) =>
      command(item, detail(item.target.node), () => {
        openSurface(item.target);
      }),
    );
    const running = pluginCommands(catalogs, place).map((item) =>
      command(item, detail(item.node), () => {
        void invokePlugin(fleet, item.call).then((outcome) => {
          if (!outcome.ok) toast(problemWords(outcome), "error");
        });
      }),
    );
    const order = declarationOrder(catalogs);
    return [...running, ...opening].sort(
      (a, b) => (order.get(a.id) ?? 0) - (order.get(b.id) ?? 0),
    );
  }, [
    agent,
    all,
    book,
    fleet,
    focus,
    held,
    hereLabel,
    openSurface,
    project,
    worktree,
  ]);
}
