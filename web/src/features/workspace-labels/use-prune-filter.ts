// A picked label that no machine has any more leaves the filter, so a
// deleted label does not keep hiding every workspace (Paseo reconciles the
// same way). Only once every connected machine's catalog has loaded
// without an error: a machine still loading, or failing, is no evidence a
// label is gone, and dropping a pick cannot be undone by the page.

import { useEffect } from "react";

import { useFleet, useFleetState } from "../../context";
import { prefer } from "../../data/prefs";
import { entriesOf } from "../../data/records";
import { withoutDeleted } from "../../data/workspace-labels/filter";
import type { LabelsState } from "../../data/workspace-labels/hub";

export function usePruneFilter(
  state: LabelsState,
  picked: readonly string[],
): void {
  const fleet = useFleet();
  const statuses = useFleetState((value) => value.statuses);
  useEffect(() => {
    if (picked.length === 0) {
      return;
    }
    const catalogs = entriesOf(statuses)
      .filter(([, status]) => status.state === "connected")
      .map(([node]) => state.catalogs[node]);
    const loaded =
      catalogs.some((catalog) => catalog?.state === "online") &&
      catalogs.every(
        (catalog) =>
          catalog !== undefined &&
          catalog.state !== "loading" &&
          catalog.error === null,
      );
    if (!loaded) {
      return;
    }
    const known = catalogs.flatMap((catalog) =>
      (catalog?.labels ?? []).map((label) => label.name),
    );
    const kept = withoutDeleted(picked, known);
    if (kept.length !== picked.length) {
      void prefer(fleet, { set: "sidebar_labels", value: { picks: kept } });
    }
  }, [fleet, statuses, state.catalogs, picked]);
}
