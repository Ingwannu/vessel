// Paseo's usage modal (usage/usage-modal.tsx), opened from the sidebar
// summary: one machine's plan usage, with a host picker when more than one
// machine is connected (N2's quotaModalHost keeps a connected pick that
// cannot report, so the card can say it needs an update). Picking a host
// is the person's choice, kept by the bridge (ADR 0107).
//
// The body is the Settings section itself (PlanUsage: Refresh all, options,
// cards, consent), so the two can never read or act differently; the
// dialog's own title is Paseo's "Usage".
import { useState } from "react";

import { useFleetState } from "../../context";
import { quotaModalHost } from "../../data/account-usage/hosts";
import { nodeLabel } from "../../data/nodes";
import { Dialog, Select } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import { PlanUsage } from "./plan-usage";
import { useUsageHosts } from "./use-usage-hosts";
import { useUsagePrefs } from "./use-usage-prefs";

export function UsageModal({
  initial,
  onClose,
}: {
  /** The host the summary showed. */
  initial: NodeKey;
  onClose: () => void;
}) {
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const { hosts, active, picked } = useUsageHosts();
  const { change } = useUsagePrefs();
  const [chosen, setChosen] = useState<NodeKey | null>(null);
  const node = chosen ?? quotaModalHost({ picked, active, hosts }) ?? initial;
  const connected = (book ?? []).filter(
    (entry) => statuses[entry.key]?.state === "connected",
  );
  const entry = (book ?? []).find((candidate) => candidate.key === node);
  const label = entry === undefined ? node : nodeLabel(entry, statuses[node]);
  return (
    <Dialog title="Usage" size="lg" onClose={onClose}>
      {connected.length > 1 ? (
        <div className="usage-modal-host">
          <Select
            aria-label="Usage host"
            size="sm"
            width="fit"
            value={node}
            options={connected.map((candidate) => ({
              value: candidate.key,
              label: nodeLabel(candidate, statuses[candidate.key]),
            }))}
            onChange={(key) => {
              setChosen(key);
              const status = statuses[key];
              if (status?.state === "connected") {
                change({ set: "usage_host", value: status.node.id });
              }
            }}
          />
        </div>
      ) : null}
      <PlanUsage node={node} label={label} />
    </Dialog>
  );
}
