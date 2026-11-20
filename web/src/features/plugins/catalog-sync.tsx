// Keeps every connected machine's plugin catalog (ADR 0115, catalog and
// invoke), as Paseo's `PluginCatalogSync` keeps its registry: mounted once
// in the shell, one watcher per machine that serves the catalog.
//
// [Decision Log]
// - Purpose: the palette, the composer and Appearance draw plugin items
//   without asking a machine each time they open.
// - Choice: a watcher reads the catalog, then long-polls `plugin.watch`
//   from the catalog's generation and reads again when it moves. A failed
//   call ends the watcher: the machine's status changes then, and the
//   watcher starts again when it is back. The last catalog stays while a
//   machine is away (Paseo keeps its registry entry); a machine taken out
//   of the book is forgotten.

import { useEffect } from "react";

import { useFleet, useFleetState } from "../../context";
import {
  keepCatalog,
  keepOnly,
  readCatalog,
  speaksCatalog,
} from "../../data/plugin-catalog";
import { watchPlugins } from "../../data/plugins";
import type { NodeKey } from "../../wire/bridge";

/** One machine's watcher. */
function MachineCatalogSync({ machine }: { machine: NodeKey }) {
  const fleet = useFleet();
  useEffect(() => {
    const ended = new AbortController();
    const stopped = () => ended.signal.aborted;
    void (async () => {
      for (;;) {
        const read = await readCatalog(fleet, machine);
        if (stopped() || !read.ok) return;
        keepCatalog(machine, read.value);
        const seen = read.value.generation;
        for (;;) {
          const next = await watchPlugins(fleet, machine, seen);
          if (stopped() || !next.ok) return;
          if (next.value !== seen) break;
        }
      }
    })();
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
  useEffect(() => {
    if (book !== null) keepOnly(new Set(book.map((node) => node.key)));
  }, [book]);
  const serving = (book ?? []).filter((node) => {
    const status = statuses[node.key];
    return status?.state === "connected" && speaksCatalog(status.protocol);
  });
  return (
    <>
      {serving.map((node) => (
        <MachineCatalogSync key={node.key} machine={node.key} />
      ))}
    </>
  );
}
