// The diff-stat hub for the workspace sidebar: held while the list is
// mounted, so the long polls run only while the card can show them.

import { useEffect } from "react";

import { useFleet } from "../../context";
import { useStore } from "../../data/store";
import {
  diffStatsOf,
  type DiffStatState,
} from "../../data/workspace/diff-stat-hub";

export function useDiffStats(): DiffStatState {
  const hub = diffStatsOf(useFleet());
  useEffect(() => hub.hold(), [hub]);
  return useStore(hub.store, (value) => value);
}
