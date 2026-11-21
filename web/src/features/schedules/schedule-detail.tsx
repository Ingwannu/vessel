import { useFleetState } from "../../context";
import { labelOf } from "../../data/nodes";
import {
  cadenceLabel,
  isHeartbeat,
  type ScheduleRow,
  scheduleTarget,
  scheduleTitle,
} from "../../data/schedules/rows";
import { Button, Dialog } from "../../ui/kit";

export function ScheduleDetail({
  row,
  close,
}: {
  row: ScheduleRow;
  close: () => void;
}) {
  const sessions = useFleetState((state) => state.sessions[row.node]);
  const host = useFleetState((state) => labelOf(state, row.node));
  const { definition } = row.info;
  const heartbeat = isHeartbeat(row.info);
  return (
    <Dialog
      title={heartbeat ? "Heartbeat" : "Schedule"}
      onClose={close}
      footer={<Button onClick={close}>Close</Button>}
    >
      <dl className="schedule-fields">
        {heartbeat ? null : (
          <div>
            <dt>Name</dt>
            <dd>{scheduleTitle(row.info)}</dd>
          </div>
        )}
        <div>
          <dt>Host</dt>
          <dd>{host}</dd>
        </div>
        <div>
          <dt>Target</dt>
          <dd>{scheduleTarget(row.info, sessions ?? [], null).label}</dd>
        </div>
        <div>
          <dt>Cadence</dt>
          <dd>{cadenceLabel(definition.cadence)}</dd>
        </div>
        {heartbeat ? null : (
          <>
            <div>
              <dt>Prompt</dt>
              <dd>{definition.prompt}</dd>
            </div>
            <div>
              <dt>Max runs</dt>
              <dd>{definition.max_runs ?? "Unlimited"}</dd>
            </div>
            {definition.target.kind === "new_session" &&
            definition.target.after !== "unknown" ? (
              <div>
                <dt>Archive on finish</dt>
                <dd>{definition.target.after === "archive" ? "On" : "Off"}</dd>
              </div>
            ) : null}
          </>
        )}
      </dl>
    </Dialog>
  );
}
