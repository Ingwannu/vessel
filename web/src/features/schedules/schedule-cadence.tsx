import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import { cadenceLabel, CRON_PRESETS } from "../../data/schedules/rows";
import { FieldGroup, Select, TextField } from "../../ui/kit";
import type { NodeKey, NodeStatus } from "../../wire/bridge";
import type { Cadence, SchedulePreview } from "../../wire/proto";

export const DEFAULT_CRON = "0 * * * *";
const PREVIEW_PROTOCOL = 13;
const PREVIEW_DELAY = 250;

export function scheduleZone(connection: NodeStatus | undefined): string {
  const device = Intl.DateTimeFormat().resolvedOptions().timeZone;
  return device !== ""
    ? device
    : connection?.state === "connected"
      ? (connection.node.timezone ?? "")
      : "";
}

type Preview =
  | { kind: "unavailable" }
  | { kind: "invalid"; message: string }
  | { kind: "loading" }
  | { kind: "ready"; result: SchedulePreview };

export function useCadencePreview(
  node: NodeKey,
  connection: NodeStatus | undefined,
  expression: string,
  zone: string,
  wanted: boolean,
): Preview {
  const fleet = useFleet();
  const expr = expression.trim().replace(/\s+/gu, " ");
  const inputError =
    zone === ""
      ? "No time zone is available for this host."
      : expr.split(" ").length === 5
        ? null
        : "Enter a five-field cron expression.";
  const supported =
    connection?.state === "connected" &&
    connection.protocol >= PREVIEW_PROTOCOL;
  const key = JSON.stringify([node, expr, zone]);
  const [answer, setAnswer] = useState<{
    key: string;
    connection: NodeStatus;
    preview: Preview;
  } | null>(null);

  useEffect(() => {
    if (!wanted || !supported || inputError !== null) return undefined;
    let current = true;
    const timer = setTimeout(() => {
      void fleet.bridge
        .call(node, "schedule.preview", {
          cadence: { kind: "cron", expr, zone },
          count: 1,
        })
        .then(
          (result) => {
            if (current)
              setAnswer({
                key,
                connection,
                preview: { kind: "ready", result },
              });
          },
          (error: unknown) => {
            const message =
              error instanceof Error
                ? error.message
                : "Could not preview this cadence.";
            if (current)
              setAnswer({
                key,
                connection,
                preview: { kind: "invalid", message },
              });
          },
        );
    }, PREVIEW_DELAY);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [fleet, node, connection, key, expr, zone, wanted, supported, inputError]);

  if (!wanted) return { kind: "unavailable" };
  if (inputError !== null) return { kind: "invalid", message: inputError };
  if (!supported) return { kind: "unavailable" };
  return answer?.key === key && answer.connection === connection
    ? answer.preview
    : { kind: "loading" };
}

function NextRun({ instant, zone }: { instant: string; zone: string }) {
  const date = new Date(instant);
  const format = (timeZone: string) =>
    new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
      timeZone,
    }).format(date);
  let text: string;
  try {
    text = format(zone);
  } catch {
    text = format("UTC");
    return <time dateTime={instant}>{text} UTC</time>;
  }
  return (
    <time dateTime={instant}>
      {text} {zone}
    </time>
  );
}

export function ScheduleCadence({
  cadence,
  preview,
  change,
  disabled,
  storedElapsed = null,
}: {
  cadence: Cadence;
  preview: Preview;
  change: (expression: string) => void;
  disabled: boolean;
  storedElapsed?: Extract<Cadence, { kind: "every" }> | null;
}) {
  if (cadence.kind !== "cron") {
    return (
      <FieldGroup label="Cadence">
        <span>{cadenceLabel(cadence)}</span>
      </FieldGroup>
    );
  }
  const expression = cadence.expr.trim();
  const activePreset = CRON_PRESETS.find(
    (preset) => preset.value === expression,
  );
  const instant = preview.kind === "ready" ? preview.result.runs[0] : undefined;
  return (
    <FieldGroup
      label="Cadence"
      error={preview.kind === "invalid" ? preview.message : null}
    >
      <Select
        aria-label="Cadence"
        value={activePreset?.value ?? null}
        options={CRON_PRESETS}
        onChange={change}
        placeholder="Custom cron"
        disabled={disabled}
      />
      <TextField
        aria-label="Cron expression"
        value={cadence.expr}
        onChange={change}
        placeholder="0 9 * * *"
        mono
        disabled={disabled}
      />
      {preview.kind === "invalid" ? null : (
        <span className="schedule-form-note" aria-live="polite">
          {instant !== undefined ? (
            <>
              {cadenceLabel(cadence)} · Next run{" "}
              <NextRun instant={instant} zone={cadence.zone} />
            </>
          ) : preview.kind === "loading" ? (
            "Checking cadence…"
          ) : expression === "" && storedElapsed !== null ? (
            cadenceLabel(storedElapsed)
          ) : (
            cadence.zone
          )}
        </span>
      )}
    </FieldGroup>
  );
}
