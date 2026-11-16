// The book's machines as usage report hosts (N2's QuotaHost), and the
// host the person picked, mapped from its node id (ADR 0107) to the book.
import { useFleetState } from "../../context";
import { servesAccountReports } from "../../data/account-usage/calls";
import type { QuotaHost } from "../../data/account-usage/hosts";
import type { NodeKey } from "../../wire/bridge";
import { useUsagePrefs } from "./use-usage-prefs";

/** The book's machines as report hosts, and the one the summary follows. */
export function useUsageHosts(): {
  hosts: QuotaHost[];
  active: NodeKey | null;
  picked: NodeKey | null;
} {
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const active = useFleetState((state) => state.selected);
  const host = useUsagePrefs().usage.host;
  const hosts = (book ?? []).map((node): QuotaHost => {
    const status = statuses[node.key];
    return {
      key: node.key,
      kind:
        status?.state !== "connected"
          ? "offline"
          : servesAccountReports(status)
            ? "supported"
            : "unsupported",
    };
  });
  const picked =
    host === null
      ? null
      : ((book ?? []).find((node) => {
          const status = statuses[node.key];
          return status?.state === "connected" && status.node.id === host;
        })?.key ?? null);
  return { hosts, active, picked };
}
