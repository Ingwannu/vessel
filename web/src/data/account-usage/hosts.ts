// A device's host pick is routing preference, never proof of an account.
// Supported means connected + report capability, not that external lookup
// is enabled. This pure choice owns no persistence, discovery or request.
import type { NodeKey } from "../../wire/bridge";

export interface QuotaHost {
  readonly key: NodeKey;
  readonly kind: "offline" | "unsupported" | "supported";
}

export interface QuotaHostChoice {
  readonly picked: NodeKey | null;
  readonly active: NodeKey | null;
  readonly hosts: readonly QuotaHost[];
}

function reports(host: QuotaHost): boolean {
  switch (host.kind) {
    case "supported":
      return true;
    case "offline":
    case "unsupported":
      return false;
  }
}

function connected(host: QuotaHost): boolean {
  switch (host.kind) {
    case "offline":
      return false;
    case "supported":
    case "unsupported":
      return true;
  }
}

/** Sidebar: picked reporting host, then active reporting host, then first. */
export function quotaSummaryHost(choice: QuotaHostChoice): NodeKey | null {
  const eligible = choice.hosts.filter(reports);
  const find = (key: NodeKey | null) =>
    eligible.find((host) => host.key === key);
  return (
    (find(choice.picked) ?? find(choice.active) ?? eligible.at(0))?.key ?? null
  );
}

/** Modal preserves a connected unsupported pick so it can explain the upgrade. */
export function quotaModalHost(choice: QuotaHostChoice): NodeKey | null {
  const available = choice.hosts.filter(connected);
  const picked = available.find((host) => host.key === choice.picked);
  return (
    picked?.key ?? quotaSummaryHost(choice) ?? available.at(0)?.key ?? null
  );
}
