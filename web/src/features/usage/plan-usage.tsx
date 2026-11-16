// Paseo's "Plan usage" section (usage/usage-section.tsx, controls.tsx,
// options.tsx) at the top of Settings > Usage: the title with Refresh all
// and the options cog trailing it, the saved-account caveat once, then one
// card per report; the source consent rows follow the cards (ADR 0042).
// The reported-usage ledger stays below, as on the native app.
import "./quota-card.css";

import { type ReactNode, useState } from "react";

import { useFleetState } from "../../context";
import { servesAccountReports } from "../../data/account-usage/calls";
import { quotaCards } from "../../data/account-usage/cards";
import type { QuotaInstant } from "../../data/account-usage/clock";
import { consentRows } from "../../data/account-usage/consent";
import type {
  AccountReader,
  AccountRead,
} from "../../data/account-usage/reader";
import type { QuotaDisplay } from "../../data/account-usage/values";
import {
  Banner,
  Button,
  type Choice,
  IconButton,
  Popover,
  Segmented,
  Switch,
  Tooltip,
} from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import { SettingsSection } from "../settings/settings-page";
import { QuotaCardView } from "./quota-card";
import { useAccountReader, useQuotaClock } from "./use-account-usage";

const TITLE = "Plan usage";
const CAVEAT =
  "These are the configured saved account’s limits, not proof of the login held by a running agent.";
const DISPLAY: readonly Choice<QuotaDisplay>[] = [
  { value: "used", label: "Used" },
  { value: "remaining", label: "Remaining" },
];

/** Paseo usage-section.tsx `UsageMessage`. */
function Message({ text, children }: { text: string; children?: ReactNode }) {
  return (
    <div className="quota-message">
      <span>{text}</span>
      {children}
    </div>
  );
}

export function PlanUsage({ node, label }: { node: NodeKey; label: string }) {
  const status = useFleetState((state) => state.statuses[node]);
  const serves = servesAccountReports(status);
  const { reader, read } = useAccountReader(node, serves, null);
  const [display, setDisplay] = useState<QuotaDisplay>("used");
  const { now, locale, deviceClock } = useQuotaClock();

  if (status?.state !== "connected") return null;
  if (!serves) {
    return (
      <SettingsSection title={TITLE}>
        <Message text={`Update ${label} to see plan usage.`} />
      </SettingsSection>
    );
  }
  const ready = read.kind === "ready" && read.node === node ? read : null;
  const on = ready?.reports.policy === "on";
  const options = (
    <Popover
      title="Usage options"
      align="end"
      trigger={<IconButton icon="settings" label="Usage options" />}
    >
      <div className="quota-options">
        <Switch
          label="Check account limits"
          showLabel
          checked={on}
          disabled={ready === null}
          onChange={(checked) => {
            reader.setLookup(checked ? "on" : "off");
          }}
        />
        <p className="quota-card-quiet">
          Allow this machine to read configured agents’ saved sign-ins and send
          them only to their vendors to check account limits. Only an
          administrator can change this setting.
        </p>
        <Segmented
          label="Display as"
          value={display}
          options={DISPLAY}
          onChange={setDisplay}
        />
      </div>
    </Popover>
  );
  const trailing = (
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
      {options}
    </>
  );
  return (
    <SettingsSection title={TITLE} action={trailing}>
      {ready === null ? (
        read.kind === "failed" ? (
          <Banner tone="error">{read.problem.message}</Banner>
        ) : (
          <Message text="Reading usage…" />
        )
      ) : (
        <PlanBody
          ready={ready}
          label={label}
          display={display}
          locale={locale}
          now={now}
          deviceClock={deviceClock}
          reader={reader}
        />
      )}
    </SettingsSection>
  );
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
  const cards = quotaCards(ready.reports, display, locale, now);
  const consent = consentRows(ready.reports);
  return (
    <>
      <p className="quota-caveat">{CAVEAT}</p>
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
