// Keeps every connected machine's plugin catalog (ADR 0115, catalog and
// invoke), as Paseo's `PluginCatalogSync` keeps its registry: mounted once
// in the shell, one watcher per machine that serves the catalog.
//
// [Decision Log]
// - Purpose: the palette, the composer and Appearance draw plugin items
//   without asking a machine each time they open.
// - Constraint: a dropped socket leaves the machine's status "connected"
//   (node review R1 on Settings › Plugins), so a watcher that ended on a
//   failed call would never start again.
// - Choice: each machine is followed as Settings › Plugins follows it
//   (`followPlugins`): a baseline from `plugin.watch`, a catalog read, then
//   the long poll; a quiet answer is paced, and a failed poll or read waits
//   `RETRY_MS` and starts over. The watchers mount only while the page's
//   link is open, so a reopened link reads every catalog afresh.
// - The last catalog stays while a machine is away (Paseo keeps its
//   registry entry); a machine taken out of the book is forgotten, and a
//   read that answers after its watcher ended is not kept.
// - Cost: one long poll per connected machine that serves the catalog,
//   and after a failure one retry every 5 s.

import { useEffect } from "react";

import { useFleet, useFleetState } from "../../context";
import {
  keepCatalog,
  keepOnly,
  readCatalog,
  speaksCatalog,
} from "../../data/plugin-catalog";
import { watchPlugins } from "../../data/plugins";
import { followPlugins } from "../../data/plugins-follow";
import { halted } from "../../data/workspace/pace";
import type { NodeKey } from "../../wire/bridge";

/** One machine's watcher. */
function MachineCatalogSync({ machine }: { machine: NodeKey }) {
  const fleet = useFleet();
  useEffect(() => {
    const ended = new AbortController();
    void followPlugins(
      {
        reload: async () => {
          const read = await readCatalog(fleet, machine);
          if (!read.ok) return false;
          if (!halted(ended.signal)) keepCatalog(machine, read.value);
          return true;
        },
        poll: (after) => watchPlugins(fleet, machine, after),
      },
      ended.signal,
    );
    return () => {
      ended.abort();
    };
  }, [fleet, machine]);
  return null;
}

/** Every connected machine's watcher, in the shell. */
export function PluginCatalogSync() {
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const open = useFleetState((state) => state.link.kind === "open");
  useEffect(() => {
    if (book !== null) keepOnly(new Set(book.map((node) => node.key)));
  }, [book]);
  const serving = open
    ? (book ?? []).filter((node) => {
        const status = statuses[node.key];
        return status?.state === "connected" && speaksCatalog(status.protocol);
      })
    : [];
  return (
    <>
      {serving.map((node) => (
        <MachineCatalogSync key={node.key} machine={node.key} />
      ))}
    </>
  );
}
