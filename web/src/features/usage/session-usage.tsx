// The account a session runs under, below its context figures (Paseo
// usage/agent-usage.tsx and components/context-window-details.tsx): one
// compact card per report, no pins. Nothing on a machine that cannot
// report (protocol < 26), or when the session has no account to report.
//
// [Decision Log] (2026-11-16, impl U, TRACKER 771 P3)
// - Purpose: Paseo's context window hover card shows the session's plan
//   usage beside how full the context is.
// - Constraint: the node decides which account a session uses
//   (`usage.reports` with a session scope); the page never infers it. The
//   read is cached-only and starts when the card opens (the popup mounts
//   this), so hovering never contacts a vendor.
// - Choice: the same reader and cards as Settings, opened with the session
//   scope; a failed read shows Paseo's one line instead of partial cards,
//   which would pass for the complete account.
import { useFleetState } from "../../context";
import { servesAccountReports } from "../../data/account-usage/calls";
import { quotaCards } from "../../data/account-usage/cards";
import type { NodeKey } from "../../wire/bridge";
import type { SessionId } from "../../wire/proto";
import { QuotaCardView } from "./quota-card";
import { useAccountReader, useQuotaClock } from "./use-account-usage";
import { useUsagePrefs } from "./use-usage-prefs";

export function SessionUsage({
  node,
  session,
}: {
  node: NodeKey;
  session: SessionId;
}) {
  const status = useFleetState((state) => state.statuses[node]);
  const serves = servesAccountReports(status);
  const { reader, read } = useAccountReader(node, serves, session);
  const { now, locale, deviceClock } = useQuotaClock();
  const display = useUsagePrefs().usage.display;
  if (!serves) return null;
  if (read.kind === "failed") {
    return (
      <div className="session-usage">
        <p className="quota-card-quiet">
          {`Unable to load usage: ${read.problem.message}`}
        </p>
      </div>
    );
  }
  if (read.kind !== "ready") {
    return (
      <div className="session-usage">
        <p className="quota-card-quiet">Loading usage...</p>
      </div>
    );
  }
  if (read.reports.policy === "off") return null;
  const cards = quotaCards(read.reports, display, locale, now);
  if (cards.length === 0) return null;
  return (
    <div className="session-usage">
      {read.problem === null ? null : (
        <p className="quota-card-error">
          {`Unable to load usage: ${read.problem.message}`}
        </p>
      )}
      {cards.map((card) => (
        <QuotaCardView
          key={card.key}
          card={card}
          display={display}
          locale={locale}
          now={now}
          deviceClock={deviceClock}
          compact
          onRefresh={
            card.refreshable
              ? () => {
                  reader.refreshOne(card.report);
                }
              : undefined
          }
        />
      ))}
    </div>
  );
}
