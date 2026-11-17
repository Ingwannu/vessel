// Specimens only, never account data: review the shared control before
// any Usage screen consumes it. The view constructs every paint value.
import "./quota-meters.css";

import { QuotaPercent } from "../../quota-percent";
import { QuotaMeter, type QuotaMeterTone } from "../../ui/kit";
import { Section } from "./section";

const INPUTS = [0, 50, 100, 125] as const;
const TONES: readonly QuotaMeterTone[] = ["default", "ok", "warning", "danger"];

export function QuotaMeters() {
  const half = QuotaPercent.from(50);
  return (
    <Section title="Quota meters">
      <p className="kit-note">
        Read-only control specimens · 읽기 전용 표시 예제
      </p>
      <div className="kit-quota-grid">
        {INPUTS.map((input) => {
          const value = QuotaPercent.from(input);
          if (value === null) return null;
          const text = value.format(navigator.language);
          return (
            <div
              className="kit-quota-specimen"
              key={input}
              data-quota-input={input}
            >
              <strong>
                {String(input)} input → {text}
              </strong>
              <span>Normal / 일반</span>
              <QuotaMeter
                value={value}
                label={`Normal ${String(input)} input`}
                valueText={text}
              />
              <span>Quiet / 낮은 강조</span>
              <QuotaMeter
                value={value}
                label={`Quiet ${String(input)} input`}
                valueText={text}
                emphasis="quiet"
                tone="danger"
              />
            </div>
          );
        })}
      </div>
      <div className="kit-quota-tones">
        {half === null
          ? null
          : TONES.map((tone) => (
              <div className="kit-quota-specimen" key={tone}>
                <strong>{tone}</strong>
                <QuotaMeter value={half} label={`${tone} tone`} tone={tone} />
              </div>
            ))}
      </div>
    </Section>
  );
}
