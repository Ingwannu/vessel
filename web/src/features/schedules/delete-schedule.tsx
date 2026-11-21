import {
  isHeartbeat,
  type ScheduleRow,
  scheduleTitle,
} from "../../data/schedules/rows";
import { Button, Dialog } from "../../ui/kit";

export function DeleteSchedule({
  row,
  close,
  confirm,
}: {
  row: ScheduleRow;
  close: () => void;
  confirm: () => void;
}) {
  return (
    <Dialog
      title={isHeartbeat(row.info) ? "Delete heartbeat" : "Delete schedule"}
      sheet="content"
      onClose={close}
      footer={
        <>
          <Button onClick={close}>Cancel</Button>
          <Button variant="danger-solid" onClick={confirm}>
            Delete
          </Button>
        </>
      }
    >
      <p>Delete “{scheduleTitle(row.info)}”? This cannot be undone.</p>
    </Dialog>
  );
}
