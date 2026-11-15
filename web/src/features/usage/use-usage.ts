// One mounted reader owns request coalescing and the bounded historical
// name catalogue. Changing scope queues a new identity, not a new reader.
import { useEffect, useMemo } from "react";

import { useFleet, useFleetState } from "../../context";
import { useStore } from "../../data/store";
import { readUsage, type UsageRange } from "../../data/usage/calls";
import { UsageReader, usageFor, type UsageRead } from "../../data/usage/reader";
import type { RollingUsageScope, UsageNames } from "../../data/usage/scope";
import type { NodeKey, NodeStatus } from "../../wire/bridge";

interface UsageView {
  status: NodeStatus | undefined;
  read: UsageRead;
  names: UsageNames | null;
  refresh: () => void;
}

export function useUsage(
  node: NodeKey,
  period: UsageRange,
  scope: RollingUsageScope,
): UsageView {
  const fleet = useFleet();
  const status = useFleetState((state) => state.statuses[node]);
  const connected = status?.state === "connected";
  const reader = useMemo(
    () =>
      new UsageReader((selection) =>
        readUsage(fleet, selection.node, selection.period, selection.scope),
      ),
    [fleet],
  );
  const read = useStore(reader.store, (value) => value);
  const names = useStore(reader.names, (value) => value);
  useEffect(
    () => () => {
      reader.stop();
    },
    [reader],
  );
  useEffect(() => {
    if (connected) reader.request({ node, period, scope });
    else reader.stop();
  }, [reader, node, period, scope, connected]);
  return {
    status,
    read: usageFor(read, { node, period, scope }),
    names: names?.node === node ? names : null,
    refresh: () => {
      reader.request({ node, period, scope });
    },
  };
}
