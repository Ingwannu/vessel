// A validated presentation value shared by data projections and the kit.
// It imports no layer: the view constructs it, and a read-only meter paints
// it without a second clamp or a dependency back into application data.

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
