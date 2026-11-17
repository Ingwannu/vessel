// Paseo's usage modal (usage/usage-modal.tsx), opened from the sidebar
// summary: "Usage" with its actions in the title row (the host filter when
// more than one machine is connected, Refresh all, the options cog), and
// the picked machine's cards alone below; the "Plan usage" heading and its
// caveat belong to the Settings page. N2's quotaModalHost keeps a
// connected pick that cannot report, so the body can say it needs an
// update. Picking a host is the person's choice, kept by the bridge
// (ADR 0107).
//
// The reader, controls and content are the Settings section's
// (usePlanUsage, PlanControls, PlanContent), so the two never read or act
// differently.
import { useState } from "react";

import { useFleetState } from "../../context";
import { quotaModalHost } from "../../data/account-usage/hosts";
import { nodeLabel } from "../../data/nodes";
import { Dialog } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import {
  Message,
  PlanContent,
  PlanControls,
  updateText,
  usePlanUsage,
} from "./plan-usage";
import { UsageHostFilter } from "./usage-host-filter";
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
  const plan = usePlanUsage(node);
  const connected = (book ?? []).filter(
    (entry) => statuses[entry.key]?.state === "connected",
  );
  const entry = (book ?? []).find((candidate) => candidate.key === node);
  const label = entry === undefined ? node : nodeLabel(entry, statuses[node]);
  const actions = (
    <>
      {connected.length > 1 ? (
        <UsageHostFilter
          hosts={connected.map((candidate) => ({
            key: candidate.key,
            name: nodeLabel(candidate, statuses[candidate.key]),
          }))}
          value={node}
          onChange={(key) => {
            setChosen(key);
            const status = statuses[key];
            if (status?.state === "connected") {
              change({ set: "usage_host", value: status.node.id });
            }
          }}
        />
      ) : null}
      {plan.connected && plan.serves ? <PlanControls plan={plan} /> : null}
    </>
  );
  return (
    <Dialog title="Usage" size="lg" onClose={onClose} actions={actions}>
      {!plan.connected ? (
        <Message text="No connected hosts" />
      ) : plan.serves ? (
        <PlanContent plan={plan} label={label} />
      ) : (
        <Message text={updateText(label)} />
      )}
    </Dialog>
  );
}
