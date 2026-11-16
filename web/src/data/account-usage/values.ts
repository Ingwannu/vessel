// Presentation values, independent of account identity and transport. The
// wire adapter will supply reported fields; this module never aggregates
// account balances, estimates cost, or decides which account a session uses.
export type QuotaDisplay = "used" | "remaining";
export type QuotaTone = "default" | "ok" | "warning" | "danger";
export type QuotaUnit = "usd" | "credits" | "requests" | "tokens";

/** A finite, clamped percentage for paint and localized display. Unknown
 * is represented by null, never by a known zero. */
export class QuotaPercent {
  readonly #value: number;

  private constructor(value: number) {
    this.#value = value;
  }

  static from(value: number): QuotaPercent | null {
    return Number.isFinite(value)
      ? new QuotaPercent(Math.max(0, Math.min(100, value)))
      : null;
  }

  get percent(): number {
    return this.#value;
  }

  get fraction(): number {
    return this.#value / 100;
  }

  format(locale: string): string {
    return new Intl.NumberFormat(locale, {
      style: "percent",
      maximumFractionDigits: 0,
    }).format(this.fraction);
  }
}

/** Independent vendor observations: a reported side takes precedence
 * over deriving its complement, even when both sides are present. */
export interface QuotaSides {
  used: number | null;
  remaining: number | null;
}

export function quotaPercent(
  sides: QuotaSides,
  display: QuotaDisplay,
): QuotaPercent | null {
  const own = display === "used" ? sides.used : sides.remaining;
  if (own !== null) return QuotaPercent.from(own);
  const other = display === "used" ? sides.remaining : sides.used;
  return other === null ? null : QuotaPercent.from(100 - other);
}

/** Source tone wins, including explicit default. Paseo's fallback becomes
 * warning at 70, danger strictly above 90, and otherwise stays muted. */
export function quotaTone(
  sides: QuotaSides,
  reported: QuotaTone | null,
): QuotaTone {
  if (reported !== null) return reported;
  const used = quotaPercent(sides, "used");
  if (used === null) return "default";
  if (used.percent > 90) return "danger";
  return used.percent >= 70 ? "warning" : "default";
}

/** A presentation result forces the renderer to distinguish absent and
 * numerically unrepresentable values from a reported amount of zero. */
export type QuotaAmount =
  | { kind: "unreported" }
  | { kind: "unrepresentable" }
  | { kind: "reported"; text: string };

export function quotaAmount(
  value: number | null,
  unit: QuotaUnit,
  locale: string,
): QuotaAmount {
  if (value === null) return { kind: "unreported" };
  if (!Number.isFinite(value) || Math.abs(value) > Number.MAX_SAFE_INTEGER) {
    return { kind: "unrepresentable" };
  }
  switch (unit) {
    case "usd":
      return {
        kind: "reported",
        text: new Intl.NumberFormat(locale, {
          style: "currency",
          currency: "USD",
        }).format(value),
      };
    case "tokens":
      return {
        kind: "reported",
        text: new Intl.NumberFormat(locale, {
          notation: "compact",
          maximumFractionDigits: 1,
        }).format(value),
      };
    case "credits":
    case "requests":
      return {
        kind: "reported",
        text: new Intl.NumberFormat(locale, {
          maximumFractionDigits: 2,
        }).format(value),
      };
  }
}
