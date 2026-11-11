// Revoking a device, after saying what that does: it loses access to the
// machine now and its open connections close; pairing again needs a new
// link.

import type { DeviceInfo } from "../../wire/proto";
import { Button, Dialog } from "../../ui/kit";

export function RevokeDialog({
  device,
  label,
  onRevoke,
  onClose,
}: {
  device: DeviceInfo;
  /** The machine's name. */
  label: string;
  onRevoke: () => void;
  onClose: () => void;
}) {
  return (
    <Dialog
      title={`Revoke ${device.label}?`}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="danger-solid" onClick={onRevoke}>
            Revoke
          </Button>
        </>
      }
    >
      <p className="dialog-copy">
        {device.label} loses access to {label} now, and its open connections
        close. Pairing it again needs a new link.
      </p>
    </Dialog>
  );
}
