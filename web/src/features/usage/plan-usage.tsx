// Paseo's "Plan usage" section (usage/usage-section.tsx, controls.tsx,
// options.tsx) at the top of Settings > Usage: the title with Refresh all
// and the options cog trailing it, the saved-account caveat once, then one
// card per report; the source consent rows follow the cards (ADR 0042).
// The reported-usage ledger stays below, as on the native app. The usage
// modal takes the same reader, controls and content (usePlanUsage,
// PlanControls, PlanContent) without the heading and caveat, as Paseo's
// modal shows the cards alone under its own title row.
import "./quota-card.css";

import type { ReactNode } from "react";

import { useFleetState } from "../../context";
import { servesAccountReports } from "../../data/account-usage/calls";
import { type QuotaCard, quotaCards } from "../../data/account-usage/cards";
import type { QuotaInstant } from "../../data/account-usage/clock";
import { consentRows } from "../../data/account-usage/consent";
import { isPinned, togglePin } from "../../data/account-usage/pins";
import type {
  AccountReader,
  AccountRead,
} from "../../data/account-usage/reader";
import type { QuotaDisplay } from "../../data/account-usage/values";
import { Banner, Button, IconButton, Tooltip } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import { SettingsSection } from "../settings/settings-page";
import { QuotaCardView, type WindowPins } from "./quota-card";
import { useAccountReader, useQuotaClock } from "./use-account-usage";
import { useUsagePrefs } from "./use-usage-prefs";
import { UsageOptions } from "./usage-options";

const TITLE = "Plan usage";
const CAVEAT =
  "These are the configured saved account’s limits, not proof of the login held by a running agent.";

/** Paseo usage-section.tsx `UsageMessage`. */
export function Message({
  text,
  children,
}: {
  text: string;
  children?: ReactNode;
}) {
  return (
    <div className="quota-message">
      <span>{text}</span>
      {children}
    </div>
  );
}

/** One machine's plan usage: its reader and whether it can report. The
 * Settings section and the usage modal share it, so the two never read or
 * act differently. */
export function usePlanUsage(node: NodeKey) {
  const status = useFleetState((state) => state.statuses[node]);
  const serves = servesAccountReports(status);
  const { reader, read } = useAccountReader(node, serves, null);
  const ready = read.kind === "ready" && read.node === node ? read : null;
  return {
    connected: status?.state === "connected",
    serves,
    reader,
    read,
    ready,
  };
}
export type PlanUsageState = ReturnType<typeof usePlanUsage>;

/** Paseo controls.tsx: Refresh all while the lookup is on, then the
 * options cog (the Settings title's trailing actions, the modal's title
 * row). */
export function PlanControls({ plan }: { plan: PlanUsageState }) {
  const { ready, reader } = plan;
  const on = ready?.reports.policy === "on";
  return (
    <>
      {ready !== null && on ? (
        <Tooltip label="Refresh all accounts">
          <IconButton
            icon="refresh"
            label="Refresh all accounts"
            disabled={ready.refreshing}
            onClick={() => {
              reader.refreshAll();
            }}
          />
        </Tooltip>
      ) : null}
      <UsageOptions
        lookup={
          ready === null
            ? null
            : {
                on,
                set: (next) => {
                  reader.setLookup(next ? "on" : "off");
                },
              }
        }
      />
    </>
  );
}

/** Paseo usage-section.tsx `UsageBody`: the cards (and our consent rows),
 * or the one message for a read that has not come or has failed. */
export function PlanContent({
  plan,
  label,
}: {
  plan: PlanUsageState;
  label: string;
}) {
  const display = useUsagePrefs().usage.display;
  const { now, locale, deviceClock } = useQuotaClock();
  if (plan.ready === null) {
    return plan.read.kind === "failed" ? (
      <Banner tone="error">{plan.read.problem.message}</Banner>
    ) : (
      <Message text="Reading usage…" />
    );
  }
  return (
    <PlanBody
      ready={plan.ready}
      label={label}
      display={display}
      locale={locale}
      now={now}
      deviceClock={deviceClock}
      reader={plan.reader}
    />
  );
}

/** Settings > Usage's section: the title with its controls, the caveat
 * once, then the content. */
export function PlanUsage({ node, label }: { node: NodeKey; label: string }) {
  const plan = usePlanUsage(node);
  if (!plan.connected) return null;
  if (!plan.serves) {
    return (
      <SettingsSection title={TITLE}>
        <Message text={updateText(label)} />
      </SettingsSection>
    );
  }
  return (
    <SettingsSection title={TITLE} action={<PlanControls plan={plan} />}>
      {plan.ready?.reports.policy === "on" ? (
        <p className="quota-caveat">{CAVEAT}</p>
      ) : null}
      <PlanContent plan={plan} label={label} />
    </SettingsSection>
  );
}

/** Paseo `hostUpgradeRequired`. */
export function updateText(label: string): string {
  return `Update ${label} to see plan usage.`;
}

function PlanBody({
  ready,
  label,
  display,
  locale,
  now,
  deviceClock,
  reader,
}: {
  ready: Extract<AccountRead, { kind: "ready" }>;
  label: string;
  display: QuotaDisplay;
  locale: string;
  now: QuotaInstant;
  deviceClock: Intl.DateTimeFormat;
  reader: AccountReader;
}) {
  const prefsOf = useUsagePrefs();
  const problem =
    ready.problem === null ? null : (
      <Banner tone="error">{ready.problem.message}</Banner>
    );
  if (ready.reports.policy === "off") {
    return (
      <>
        <Message text={`Usage is off on ${label}.`}>
          <Button
            variant="secondary"
            onClick={() => {
              reader.setLookup("on");
            }}
          >
            Turn on
          </Button>
        </Message>
        {problem}
      </>
    );
  }
  const { usage, inSidebar, change } = prefsOf;
  const pinsFor = (card: QuotaCard): WindowPins | undefined =>
    inSidebar
      ? {
          isPinned: (window) =>
            isPinned(
              usage.pins,
              { source: card.source, window },
              ready.reports,
              display,
            ),
          toggle: (window) => {
            change({
              set: "usage_pins",
              value: togglePin(
                usage.pins,
                { source: card.source, window },
                ready.reports,
                display,
              ),
            });
          },
        }
      : undefined;
  const cards = quotaCards(ready.reports, display, locale, now);
  const consent = consentRows(ready.reports);
  return (
    <>
      {problem}
      {cards.length === 0 ? (
        <Message text="No usage data" />
      ) : (
        <div className="quota-cards">
          {cards.map((card) => (
            <QuotaCardView
              key={card.key}
              card={card}
              display={display}
              locale={locale}
              now={now}
              deviceClock={deviceClock}
              onRefresh={
                card.refreshable
                  ? () => {
                      reader.refreshOne(card.report);
                    }
                  : undefined
              }
              pins={pinsFor(card)}
            />
          ))}
        </div>
      )}
      {consent.map((row) => (
        <div key={row.source.id} className="quota-card quota-consent">
          <div className="quota-card-head">
            <span className="quota-card-name">{row.source.label}</span>
            <span className="quota-card-spacer" />
            <Button
              size="sm"
              onClick={() => {
                reader.setSource(
                  row.source.id,
                  row.next,
                  row.source.disclosure.version,
                );
              }}
            >
              {row.action}
            </Button>
          </div>
          <p className="quota-card-quiet">{row.disclosure}</p>
          <p className="quota-card-quiet">{row.state}</p>
        </div>
      ))}
    </>
  );
}
