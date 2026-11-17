// A session's limit notice (native app areas/usage/session.rs
// `limit_notice`; Paseo has none): the node's `UsageLimits.notice` says the
// saved login the session runs under reports a full general window. The
// node decides that (vessel-node limits/notice.rs: not when the vendor
// still allows the turn, nor with credits or extra usage); the page only
// keeps the reported reset while it is still ahead, as the native app does.
import type { UsageLimits } from "../../wire/proto";
import { QuotaInstant } from "./clock";

export interface SessionNotice {
  /** The latest reported reset of the full windows, when still ahead. */
  reset: QuotaInstant | null;
}

export function sessionNotice(
  limits: UsageLimits,
  now: QuotaInstant,
): SessionNotice | null {
  const notice = limits.notice;
  if (notice === null) return null;
  const reset =
    notice.latest_reported_reset === null
      ? null
      : QuotaInstant.fromIso(notice.latest_reported_reset);
  return {
    reset:
      reset !== null && reset.milliseconds > now.milliseconds ? reset : null,
  };
}
