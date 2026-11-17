// The session's limit notice over its composer (native app
// areas/usage/session.rs `limit_notice`, in the conversation dock; Paseo
// has none). When the saved login the session runs under reports a full
// general window: the fact, said as no proof of why a turn stopped; the
// latest reported reset while it is ahead; that nothing is sent by itself;
// and Switch model, which opens the composer's own model control.
//
// [Decision Log] (2026-11-17, impl U, TRACKER 771 P3)
// - Purpose: the page says what the native app says when a session's
//   account reports its limit, from the node's own notice.
// - Constraint: the notice is only in `usage.limits` (23). It belongs to
//   the login the session ran under when it was read, which a status,
//   profile or options change, a park, a wake or a rewind may change
//   (native `binding_epoch`). The kit Menu takes no outside open, and the
//   composer and the kit menu are at their line budgets.
// - Alternatives: derive "full" from the session's reports (a second copy
//   of the node's rule, which also weighs credits and extra usage); open
//   the model menu through a controlled state in the kit and the composer.
// - Choice: a cached `usage.limits` read for the session when its composer
//   opens and again after each such record (`Timeline.binding`); a notice
//   read before one is hidden until the new answer. Switch model clicks the
//   model trigger in the same composer (`data-model-picker`), as a person's
//   click does: Base UI opens the menu and moves focus into it.
// - Cost: the button depends on that mark (features/compose/agent-menu.tsx);
//   the words are the native English ones (view/locales/en/usage.ftl).
import "./limit-notice.css";

import { useEffect, useMemo, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import {
  readSessionLimits,
  servesLimits,
} from "../../data/account-usage/calls";
import {
  type SessionNotice,
  sessionNotice,
} from "../../data/account-usage/session-notice";
import { Button, Icon } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { SessionId, UsageLimits } from "../../wire/proto";
import { useQuotaClock } from "./use-account-usage";

/** The read for one session and login record; `null` before an answer,
 * after a failure (an agent query's error hides its values, as on the
 * native app) and where the machine does not serve `usage.limits`. */
function useSessionLimits(
  node: NodeKey,
  session: SessionId,
  binding: number,
): UsageLimits | null {
  const fleet = useFleet();
  const serves = useFleetState((state) => servesLimits(state.statuses[node]));
  const key = `${node}\u0000${session}\u0000${String(binding)}`;
  const [kept, setKept] = useState<{
    key: string;
    limits: UsageLimits | null;
  } | null>(null);
  useEffect(() => {
    if (!serves) return undefined;
    let current = true;
    void readSessionLimits(fleet, node, session).then((outcome) => {
      if (current) setKept({ key, limits: outcome.ok ? outcome.value : null });
    });
    return () => {
      current = false;
    };
  }, [fleet, node, session, key, serves]);
  return serves && kept?.key === key ? kept.limits : null;
}

/** The composer's model control, pressed as a person would press it. */
export function pressModelControl(from: Element): void {
  from
    .closest(".prompt-dock")
    ?.querySelector<HTMLButtonElement>("[data-model-picker]")
    ?.click();
}

export function LimitNotice({
  node,
  session,
  binding,
}: {
  node: NodeKey;
  session: SessionId;
  /** `Timeline.binding`: read again after it. */
  binding: number;
}) {
  const limits = useSessionLimits(node, session, binding);
  const { now, locale } = useQuotaClock();
  const notice = limits === null ? null : sessionNotice(limits, now);
  if (notice === null) return null;
  return <LimitNoticeView notice={notice} locale={locale} />;
}

export function LimitNoticeView({
  notice,
  locale,
}: {
  notice: SessionNotice;
  locale: string;
}) {
  const clock = useMemo(
    () =>
      new Intl.DateTimeFormat(locale, {
        dateStyle: "medium",
        timeStyle: "short",
      }),
    [locale],
  );
  const reset =
    notice.reset === null ? null : new Date(notice.reset.milliseconds);
  return (
    <div className="limit-notice" role="status" aria-label="Usage limit">
      <span className="limit-notice-icon">
        <Icon name="warn" size={16} />
      </span>
      <div className="limit-notice-text">
        <p className="limit-notice-title">
          The saved account reports a usage limit. This does not prove why the
          turn stopped.
        </p>
        {reset === null ? null : (
          <p className="limit-notice-detail">
            <time dateTime={reset.toISOString()}>{clock.format(reset)}</time>
          </p>
        )}
        <p className="limit-notice-detail">Wait for reset</p>
        <p className="limit-notice-detail">
          No prompt will be sent automatically. Continue when the account is
          ready.
        </p>
      </div>
      <Button
        size="sm"
        shape="round"
        onClick={(event) => {
          pressModelControl(event.currentTarget);
        }}
      >
        Switch model
      </Button>
    </div>
  );
}
