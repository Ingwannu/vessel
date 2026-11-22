import { useId, useState } from "react";

import { useFleetState } from "../../context";
import {
  heartbeatRequest,
  HeartbeatSaveUnconfirmed,
} from "../../data/schedules/heartbeat";
import type { MachineSchedules } from "../../data/schedules/machine-schedules";
import { type ScheduleRow, scheduleTarget } from "../../data/schedules/rows";
import { Button, Dialog, FieldGroup } from "../../ui/kit";
import {
  ScheduleCadence,
  scheduleZone,
  useCadencePreview,
} from "./schedule-cadence";

export function ScheduleHeartbeat({
  row,
  schedules,
  available,
  close,
}: {
  row: ScheduleRow;
  schedules: MachineSchedules;
  available: boolean;
  close: () => void;
}) {
  const sessions = useFleetState((state) => state.sessions[row.node]);
  const connection = useFleetState((state) => state.statuses[row.node]);
  const formId = useId();
  const original = row.info.definition.cadence;
  const [expression, setExpression] = useState(
    original.kind === "cron" ? original.expr : "",
  );
  const [busy, setBusy] = useState(false);
  const [unconfirmed, setUnconfirmed] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const zone =
    original.kind === "cron" ? original.zone : scheduleZone(connection);
  const cadence = { kind: "cron", expr: expression, zone } as const;
  const preview = useCadencePreview(
    row.node,
    connection,
    expression,
    zone,
    available && !unconfirmed && expression.trim() !== "",
  );
  const valid =
    available &&
    !unconfirmed &&
    preview.kind === "ready" &&
    heartbeatRequest(row.info, cadence, preview.result) !== null;
  const notice =
    error ??
    (available
      ? null
      : "The original heartbeat connection is unavailable. Reopen it to edit.");

  const save = async () => {
    if (busy || !valid) return;
    setBusy(true);
    setError(null);
    try {
      await schedules.saveHeartbeat(row, cadence, preview.result);
      close();
    } catch (reason: unknown) {
      setError(
        reason instanceof Error ? reason.message : "Could not save heartbeat.",
      );
      setUnconfirmed(reason instanceof HeartbeatSaveUnconfirmed);
      setBusy(false);
    }
  };

  return (
    <Dialog
      title="Edit heartbeat"
      onClose={() => {
        if (!busy) close();
      }}
      footer={
        <>
          <Button disabled={busy} onClick={close}>
            Cancel
          </Button>
          <Button
            type="submit"
            form={formId}
            variant="primary"
            busy={busy}
            disabled={busy || !valid}
          >
            Save changes
          </Button>
        </>
      }
    >
      <form
        id={formId}
        className="schedule-form"
        onSubmit={(event) => {
          event.preventDefault();
          void save();
        }}
      >
        <FieldGroup label="Target">
          <div className="text-field schedule-target-readout">
            <span>{scheduleTarget(row.info, sessions ?? [], null).label}</span>
          </div>
        </FieldGroup>
        <ScheduleCadence
          cadence={cadence}
          preview={preview}
          change={(next) => {
            setError(null);
            setExpression(next);
          }}
          disabled={busy || !available || unconfirmed}
          storedElapsed={original.kind === "every" ? original : null}
        />
        {notice === null ? null : (
          <p className="schedule-form-error" role="alert">
            {notice}
          </p>
        )}
      </form>
    </Dialog>
  );
}
