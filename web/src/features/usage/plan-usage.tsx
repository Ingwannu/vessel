// Paseo's "Plan usage" section (usage/usage-section.tsx, controls.tsx,
// options.tsx) at the top of Settings > Usage: the title with Refresh all
// and the options cog trailing it, the saved-account caveat once, then one
// card per report; the source consent rows follow the cards (ADR 0042).
// The reported-usage ledger stays below, as on the native app.
import "./quota-card.css";

import { type ReactNode, useEffect, useMemo, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import {
  readAccountReports,
  refreshAccountReport,
  servesAccountReports,
  setAccountLookup,
  setSourceLookup,
} from "../../data/account-usage/calls";
import { quotaCards } from "../../data/account-usage/cards";
import { QuotaInstant } from "../../data/account-usage/clock";
import { consentRows } from "../../data/account-usage/consent";
import {
  AccountReader,
  type AccountRead,
} from "../../data/account-usage/reader";
import type { QuotaDisplay } from "../../data/account-usage/values";
import { useStore } from "../../data/store";
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

const TITLE = "Plan usage";
const CAVEAT =
  "These are the configured saved account’s limits, not proof of the login held by a running agent.";
const DISPLAY: readonly Choice<QuotaDisplay>[] = [
  { value: "used", label: "Used" },
  { value: "remaining", label: "Remaining" },
];

/** Date.now() is always a safe integer; anything else is a broken clock. */
function instantNow(): QuotaInstant {
  const now = QuotaInstant.fromMilliseconds(Date.now());
  if (now === null) throw new Error("The device clock is out of range");
  return now;
}

/** Paseo's relative-time tick: once a minute while cards show times. */
function useNow(): QuotaInstant {
  const [now, setNow] = useState(instantNow);
  useEffect(() => {
    const timer = window.setInterval(() => {
      setNow(instantNow());
    }, 60_000);
    return () => {
      window.clearInterval(timer);
    };
  }, []);
  return now;
}

function useAccountReader(
  node: NodeKey,
  serves: boolean,
): {
  reader: AccountReader;
  read: AccountRead;
} {
  const fleet = useFleet();
  const reader = useMemo(
    () =>
      new AccountReader({
        read: (key, refresh) => readAccountReports(fleet, key, refresh),
        inspect: (key, id) => refreshAccountReport(fleet, key, id),
        lookup: (key, policy) => setAccountLookup(fleet, key, policy),
        source: (key, source, policy, version) =>
          setSourceLookup(fleet, key, source, policy, version),
      }),
    [fleet],
  );
  const read = useStore(reader.store, (value) => value);
  useEffect(() => {
    if (serves) reader.open(node);
    else reader.stop();
    return () => {
      reader.stop();
    };
  }, [reader, node, serves]);
  return { reader, read };
}

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
  const { reader, read } = useAccountReader(node, serves);
  const [display, setDisplay] = useState<QuotaDisplay>("used");
  const now = useNow();
  const locale = navigator.language;
  const deviceClock = useMemo(
    () =>
      new Intl.DateTimeFormat(locale, { hour: "numeric", minute: "2-digit" }),
    [locale],
  );

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
