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
import { freshnessText, resetText } from "./quota-text";
import type { QuotaInstant } from "../../data/account-usage/clock";
import type { QuotaDisplay } from "../../data/account-usage/values";
import {
  Badge,
  Icon,
  IconButton,
  ProviderMark,
  QuotaMeter,
  Tooltip,
} from "../../ui/kit";

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

/** While the sidebar summary is on, a window row toggles its pin
 * (Paseo window-bar.tsx: the row is a checkbox, the glyph shows its state). */
export interface WindowPins {
  isPinned: (window: string) => boolean;
  toggle: (window: string) => void;
}

function WindowBar({
  window,
  display,
  locale,
  deviceClock,
  pins,
}: {
  window: QuotaCardWindow;
  display: QuotaDisplay;
  locale: string;
  deviceClock: Intl.DateTimeFormat;
  pins: WindowPins | undefined;
}) {
  const value =
    window.percent === null
      ? "Not reported"
      : `${window.percent.format(locale)} ${display === "used" ? "used" : "remaining"}`;
  const pinned = pins?.isPinned(window.key) ?? false;
  const body = (
    <>
      <div className="quota-bar-label">
        <span className="quota-bar-name">{window.label}</span>
        <span className="quota-bar-value">{value}</span>
        {pins === undefined ? null : (
          <span className={pinned ? "quota-pin pinned" : "quota-pin"}>
            <Icon name="pin" size={12} />
          </span>
        )}
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
    </>
  );
  if (pins === undefined) return <div className="quota-bar">{body}</div>;
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={pinned}
      aria-label={`Pin ${window.label}, ${value}`}
      className="quota-bar pinnable"
      onClick={() => {
        pins.toggle(window.key);
      }}
    >
      {body}
    </button>
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
  compact = false,
  pins,
}: {
  card: QuotaCard;
  display: QuotaDisplay;
  locale: string;
  now: QuotaInstant;
  deviceClock: Intl.DateTimeFormat;
  /** Absent where the source cannot be refreshed (ADR 0042). */
  onRefresh: (() => void) | undefined;
  /** Paseo's compact card (the session's hover card): the freshness is
   * printed on the card, as where there is no hover. */
  compact?: boolean;
  /** Window rows pin to the sidebar summary while it is on. */
  pins?: WindowPins | undefined;
}) {
  const freshness = freshnessText(card.fetchedAt, now);
  return (
    <article
      className={compact ? "quota-card compact" : "quota-card"}
      aria-label={card.name}
    >
      <header className="quota-card-head">
        {card.mark === null ? (
          <Icon name="plan" size={14} />
        ) : (
          <ProviderMark name={card.mark} size={14} />
        )}
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
              pins={pins}
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
