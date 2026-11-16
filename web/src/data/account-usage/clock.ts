// A quota's reset and projected run-out are different facts. These view
// states make it impossible to attach an exact reset time to a run-out
// warning, where a person could mistake it for the exhaustion time.
import type { QuotaPercent } from "./values";

const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;
const WEEK_MS = 7 * DAY_MS;

/** Parsed once at the presentation boundary; invalid times stay absent. */
export class QuotaInstant {
  readonly #milliseconds: number;

  private constructor(milliseconds: number) {
    this.#milliseconds = milliseconds;
  }

  static fromIso(value: string): QuotaInstant | null {
    return QuotaInstant.fromMilliseconds(Date.parse(value));
  }

  static fromMilliseconds(value: number): QuotaInstant | null {
    return Number.isSafeInteger(value) &&
      Number.isFinite(new Date(value).getTime())
      ? new QuotaInstant(value)
      : null;
  }

  get milliseconds(): number {
    return this.#milliseconds;
  }
}

type DurationUnit = "days" | "hours" | "minutes";
interface DurationPart {
  unit: DurationUnit;
  count: number;
}
type DurationParts = readonly [DurationPart, DurationPart?];
type ResetDistance = "day" | "week" | "later";

type RelativeQuotaTime =
  | { kind: "elapsed" }
  | { kind: "remaining"; parts: DurationParts; distance: ResetDistance };

function parts(major: DurationPart, minor?: DurationPart): DurationParts {
  return minor !== undefined && minor.count > 0 ? [major, minor] : [major];
}

function relative(
  at: QuotaInstant,
  now: QuotaInstant,
): RelativeQuotaTime | null {
  const remaining = at.milliseconds - now.milliseconds;
  if (!Number.isSafeInteger(remaining)) return null;
  if (remaining <= 0) return { kind: "elapsed" };
  const days = Math.floor(remaining / DAY_MS);
  const hours = Math.floor((remaining % DAY_MS) / HOUR_MS);
  const minutes = Math.floor((remaining % HOUR_MS) / MINUTE_MS);
  const distance =
    remaining < DAY_MS ? "day" : remaining < WEEK_MS ? "week" : "later";
  return {
    kind: "remaining",
    distance,
    parts:
      days > 0
        ? parts({ unit: "days", count: days }, { unit: "hours", count: hours })
        : hours > 0
          ? parts(
              { unit: "hours", count: hours },
              { unit: "minutes", count: minutes },
            )
          : parts({ unit: "minutes", count: minutes }),
  };
}

/** Only reset has an exact time. Retained past risk yields back to reset. */
export type QuotaClock =
  | { kind: "unreported" }
  | { kind: "resetting" }
  | { kind: "reset_passed" }
  | {
      kind: "reset";
      parts: DurationParts;
      at: QuotaInstant;
      distance: ResetDistance;
    }
  | { kind: "risk_now"; shortfall: QuotaPercent }
  | { kind: "risk"; parts: DurationParts; shortfall: QuotaPercent };

export type QuotaSnapshotBasis = "current" | "retained";

export interface QuotaClockInput {
  reset: QuotaInstant | null;
  runOut: QuotaInstant | null;
  shortfall: QuotaPercent | null;
  basis: QuotaSnapshotBasis;
}

export function quotaClock(
  input: QuotaClockInput,
  now: QuotaInstant,
): QuotaClock {
  const { runOut, shortfall, reset, basis } = input;
  const retained = basis === "retained";
  if (
    runOut !== null &&
    shortfall !== null &&
    (!retained || runOut.milliseconds > now.milliseconds)
  ) {
    const span = relative(runOut, now);
    if (span === null) return { kind: "unreported" };
    return span.kind === "elapsed"
      ? { kind: "risk_now", shortfall }
      : { kind: "risk", parts: span.parts, shortfall };
  }
  if (reset === null) return { kind: "unreported" };
  const span = relative(reset, now);
  if (span === null) return { kind: "unreported" };
  return span.kind === "elapsed"
    ? { kind: retained ? "reset_passed" : "resetting" }
    : { kind: "reset", parts: span.parts, at: reset, distance: span.distance };
}

/** Freshness belongs to a successful snapshot, never a failed attempt.
 * Caller supplies that successful instant; future clock skew is 'now'. */
export type QuotaAge =
  { kind: "now" } | { kind: "ago"; unit: DurationUnit; count: number };

export function quotaAge(
  at: QuotaInstant | null,
  now: QuotaInstant,
): QuotaAge | null {
  if (at === null) return null;
  const elapsed = now.milliseconds - at.milliseconds;
  if (!Number.isSafeInteger(elapsed)) return null;
  if (elapsed < MINUTE_MS) return { kind: "now" };
  if (elapsed >= DAY_MS)
    return { kind: "ago", unit: "days", count: Math.floor(elapsed / DAY_MS) };
  if (elapsed >= HOUR_MS)
    return { kind: "ago", unit: "hours", count: Math.floor(elapsed / HOUR_MS) };
  return {
    kind: "ago",
    unit: "minutes",
    count: Math.floor(elapsed / MINUTE_MS),
  };
}
