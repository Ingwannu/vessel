// The account-usage calls the page makes (protocol 26, ADR 0042): the
// machine's account reports, one report's refresh, the machine-wide lookup
// switch and one source's consent. Ledger totals stay in `data/usage`.
//
// [Decision Log] (2026-11-16, impl U, TRACKER 771)
// - Purpose: Paseo's Settings > Usage cards on the page, from the node's
//   own reports, never from the browser's guesses.
// - Constraint: the three report methods exist from protocol 26 only; an
//   older machine answers `not_in_protocol`. The page keeps today's
//   behaviour there (no account cards) and says the machine needs an update,
//   as Paseo's `hostUpgradeRequired` does.
// - Choice: the protocol is read from the connected status before calling,
//   so an older machine is never asked; `usage.limits` (23) is not wired on
//   the page, which never drew it.
// - Read is cached-only and never contacts a vendor; only Refresh (Operate)
//   and a source's consent (Admin) change anything, both explicit clicks.
import type { NodeKey, NodeStatus } from "../../wire/bridge";
import type {
  ExternalLookup,
  LimitRefresh,
  UsageDisclosureVersion,
  UsageReportId,
  UsageReportLookup,
  UsageReportScope,
  UsageReports,
  UsageSourceDescriptor,
  UsageSourceId,
} from "../../wire/proto";
import type { Fleet } from "../fleet";
import { attempt, type Outcome } from "../outcome";

/** `usage.reports`, `usage.inspect` and `usage.source_lookup` (PROTOCOL.md v26). */
export const ACCOUNT_REPORTS_PROTOCOL = 26;

/** Whether `status` is a connected machine that serves the report methods. */
export function servesAccountReports(status: NodeStatus | undefined): boolean {
  return (
    status?.state === "connected" && status.protocol >= ACCOUNT_REPORTS_PROTOCOL
  );
}

/** The machine's reports, or one session's account (Paseo's agent usage).
 * `cached` never discovers logins or contacts a source. */
export function readAccountReports(
  fleet: Fleet,
  node: NodeKey,
  refresh: LimitRefresh,
  scope: UsageReportScope,
): Promise<Outcome<UsageReports>> {
  return attempt(() =>
    fleet.bridge.call(node, "usage.reports", { scope, refresh }),
  );
}

/** One report, fetched again on an explicit Refresh (Operate). */
export function refreshAccountReport(
  fleet: Fleet,
  node: NodeKey,
  id: UsageReportId,
): Promise<Outcome<UsageReportLookup>> {
  return attempt(() =>
    fleet.bridge.call(node, "usage.inspect", { id, refresh: "refresh" }),
  );
}

/** The machine-wide lookup switch (Admin); Off clears snapshots first. */
export function setAccountLookup(
  fleet: Fleet,
  node: NodeKey,
  policy: ExternalLookup,
): Promise<Outcome<ExternalLookup>> {
  return attempt(() => fleet.bridge.call(node, "usage.lookup", { policy }));
}

/** One source's consent (Admin), against the disclosure version shown. */
export function setSourceLookup(
  fleet: Fleet,
  node: NodeKey,
  source: UsageSourceId,
  policy: ExternalLookup,
  disclosure: UsageDisclosureVersion,
): Promise<Outcome<UsageSourceDescriptor>> {
  return attempt(() =>
    fleet.bridge.call(node, "usage.source_lookup", {
      source,
      policy,
      disclosure_version: disclosure,
    }),
  );
}
