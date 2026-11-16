// One Paseo usage card (usage/card.tsx 81-160): the in-card header (mark,
// name, plan badge, status dot with its word, icon Refresh whose tooltip
// says when the values were fetched), then the window and balance bars
// (window-bar.tsx, balance-bar.tsx), the detail rows and the account label.
// Values come from `data/account-usage/cards`; this file only lays out.
import "./quota-card.css";

import type {
  QuotaCard,
  QuotaCardBalance,
  QuotaCardWindow,
} from "../../data/account-usage/cards";
import type { QuotaClock } from "../../data/account-usage/clock";
import { quotaDurationText } from "../../data/account-usage/duration";
import { quotaAge, QuotaInstant } from "../../data/account-usage/clock";
import { quotaResetClock } from "../../data/account-usage/format";
import type { QuotaDisplay } from "../../data/account-usage/values";
import { Badge, Icon, IconButton, QuotaMeter, Tooltip } from "../../ui/kit";

/** Paseo reset-line.tsx: the countdown and its exact time, or the forecast. */
export function resetText(
  clock: QuotaClock,
  locale: string,
  deviceClock: Intl.DateTimeFormat,
): { text: string; atRisk: boolean } | null {
  switch (clock.kind) {
    case "unreported":
      return null;
    case "resetting":
    case "reset_passed":
      return {
        text: "Reported reset has passed; refresh to check again.",
        atRisk: false,
      };
    case "reset": {
      const duration = quotaDurationText(clock.parts, locale);
      const at = quotaResetClock(clock, locale, deviceClock);
      const time =
        at === null
          ? null
          : at.kind === "time"
            ? at.time
            : `${at.day} ${at.time}`;
      return {
        text:
          time === null
            ? `Resets in ${duration}`
            : `Resets in ${duration} · ${time}`,
        atRisk: false,
      };
    }
    case "risk_now":
      return {
        text: `Source forecast: runs out now · Source forecast shortfall: ${clock.shortfall.format(locale)}`,
        atRisk: true,
      };
    case "risk":
      return {
        text: `Source forecast: runs out in ${quotaDurationText(clock.parts, locale)} · Source forecast shortfall: ${clock.shortfall.format(locale)}`,
        atRisk: true,
      };
  }
}

/** "Updated 10m ago": the shown success, never a failed attempt. */
export function freshnessText(
  at: QuotaInstant | null,
  now: QuotaInstant,
): string {
  const age = quotaAge(at, now);
  if (age === null) return "No successful update";
  if (age.kind === "now") return "Updated just now";
  const unit = age.unit === "days" ? "d" : age.unit === "hours" ? "h" : "m";
  return `Updated ${String(age.count)}${unit} ago`;
}

function ResetLine({
  clock,
  locale,
  deviceClock,
}: {
  clock: QuotaClock;
  locale: string;
  deviceClock: Intl.DateTimeFormat;
}) {
  const line = resetText(clock, locale, deviceClock);
  if (line === null) return null;
  return (
    <span className={line.atRisk ? "quota-reset at-risk" : "quota-reset"}>
      {line.text}
    </span>
  );
}

function WindowBar({
  window,
  display,
  locale,
  deviceClock,
}: {
  window: QuotaCardWindow;
  display: QuotaDisplay;
  locale: string;
  deviceClock: Intl.DateTimeFormat;
}) {
  const value =
    window.percent === null
      ? "Not reported"
      : `${window.percent.format(locale)} ${display === "used" ? "used" : "remaining"}`;
  return (
    <div className="quota-bar">
      <div className="quota-bar-label">
        <span className="quota-bar-name">{window.label}</span>
        <span className="quota-bar-value">{value}</span>
      </div>
      {window.percent === null ? null : (
        <QuotaMeter
          value={window.percent}
          label={window.label}
          valueText={value}
          tone={window.tone}
        />
      )}
      <ResetLine
        clock={window.clock}
        locale={locale}
        deviceClock={deviceClock}
      />
    </div>
  );
}

function BalanceBar({
  balance,
  locale,
  deviceClock,
}: {
  balance: QuotaCardBalance;
  locale: string;
  deviceClock: Intl.DateTimeFormat;
}) {
  return (
    <div className="quota-bar">
      <div className="quota-bar-label">
        <span className="quota-bar-name">{balance.label}</span>
        <span className="quota-bar-value">{balance.amount}</span>
      </div>
      {balance.percent === null ? null : (
        <QuotaMeter
          value={balance.percent}
          label={balance.label}
          valueText={balance.amount}
          tone={balance.tone}
        />
      )}
      <ResetLine
        clock={balance.clock}
        locale={locale}
        deviceClock={deviceClock}
      />
    </div>
  );
}

export function QuotaCardView({
  card,
  display,
  locale,
  now,
  deviceClock,
  onRefresh,
}: {
  card: QuotaCard;
  display: QuotaDisplay;
  locale: string;
  now: QuotaInstant;
  deviceClock: Intl.DateTimeFormat;
  /** Absent where the source cannot be refreshed (ADR 0042). */
  onRefresh: (() => void) | undefined;
}) {
  const freshness = freshnessText(card.fetchedAt, now);
  return (
    <article className="quota-card" aria-label={card.name}>
      <header className="quota-card-head">
        <Icon name="plan" size={14} />
        <span className="quota-card-name">{card.name}</span>
        {card.plan === null ? null : <Badge>{card.plan}</Badge>}
        <span className="quota-card-spacer" />
        {card.status === null ? null : (
          <span className="quota-card-status">
            <span
              className={`quota-card-dot ${card.status}`}
              aria-hidden="true"
            />
            {card.status === "error" ? "Error" : "Unavailable"}
          </span>
        )}
        {onRefresh === undefined ? null : (
          <Tooltip label={freshness}>
            <IconButton
              icon="refresh"
              label={`Refresh ${card.name}`}
              disabled={card.refreshing}
              onClick={onRefresh}
            />
          </Tooltip>
        )}
      </header>
      {card.partial ? <p className="quota-card-quiet">Partial report</p> : null}
      {card.notChecked ? (
        <p className="quota-card-quiet">
          Limits have not been checked yet. Use Refresh to check this saved
          account.
        </p>
      ) : null}
      {card.notes.map((note) => (
        // A failed attempt reads in danger ink; an unavailable source's
        // reason is quiet, as on the native card.
        <p
          key={note}
          className={
            card.status === "error" ? "quota-card-error" : "quota-card-quiet"
          }
        >
          {note}
        </p>
      ))}
      {card.windows.length + card.balances.length === 0 ? null : (
        <div className="quota-card-bars">
          {card.windows.map((window) => (
            <WindowBar
              key={window.key}
              window={window}
              display={display}
              locale={locale}
              deviceClock={deviceClock}
            />
          ))}
          {card.balances.map((balance) => (
            <BalanceBar
              key={balance.key}
              balance={balance}
              locale={locale}
              deviceClock={deviceClock}
            />
          ))}
        </div>
      )}
      {card.details.length === 0 ? null : (
        <dl className="quota-card-details">
          {card.details.map((detail) => (
            <div key={detail.key} className="quota-card-detail">
              <dt>{detail.label}</dt>
              <dd>{detail.value}</dd>
            </div>
          ))}
        </dl>
      )}
      {card.account === null ? null : (
        <footer className="quota-card-foot">
          <span className="quota-card-account">{card.account}</span>
          <span className="quota-card-fresh">{freshness}</span>
        </footer>
      )}
    </article>
  );
}
