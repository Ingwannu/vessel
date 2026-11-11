// Settings › Devices: the phones and computers paired with a machine, a
// new pairing link for one more, and revoking one (the old Devices page,
// on rows and the kit).

import { useCallback, useEffect, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import {
  deviceLine,
  listDevices,
  pairedDevices,
  revokeDevice,
} from "../../data/devices";
import { machineFor, machineState } from "../../data/machines";
import { nodeLabel } from "../../data/nodes";
import { nextStep, type Outcome } from "../../data/outcome";
import type { NodeKey } from "../../wire/bridge";
import type { DeviceInfo } from "../../wire/proto";
import { Banner, Button, Row, RowList, Skeleton } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import { MachinePicker } from "../machines/machine-picker";
import { InviteCard } from "./invite-card";
import { RevokeDialog } from "./revoke-dialog";
import { SettingsPage, SettingsSection } from "./settings-page";

const LEAD =
  "Phones and computers that can use a machine. Pair one with a link; revoke one to close its connections.";

export function DevicesSection({ machine }: { machine: string | undefined }) {
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const selected = useFleetState((state) => state.selected);
  const node = machineFor(book ?? [], machine, selected);
  if (node === null) {
    return (
      <SettingsPage lead={LEAD}>
        {book === null ? (
          <Skeleton />
        ) : (
          <p className="settings-empty">No machines yet.</p>
        )}
      </SettingsPage>
    );
  }
  return (
    <DevicesOf
      key={node.key}
      machine={node.key}
      label={nodeLabel(node, statuses[node.key])}
    />
  );
}

/** One machine's devices; keyed by the machine, so nothing of another
 * machine's list or link is ever shown under this one's name. */
function DevicesOf({ machine, label }: { machine: NodeKey; label: string }) {
  const fleet = useFleet();
  const status = useFleetState((state) => state.statuses[machine]);
  const connected = status?.state === "connected";
  const [listed, setListed] = useState<Outcome<readonly DeviceInfo[]> | null>(
    null,
  );
  const [revoking, setRevoking] = useState<DeviceInfo | null>(null);
  const load = useCallback(() => {
    void listDevices(fleet, machine).then(setListed);
  }, [fleet, machine]);
  useEffect(() => {
    if (connected) {
      load();
    }
  }, [connected, load]);
  const picker = <MachinePicker section="devices" machine={machine} />;
  if (!connected) {
    return (
      <SettingsPage lead={LEAD}>
        <SettingsSection title={`Paired with ${label}`} action={picker}>
          <p className="settings-empty">
            {label} is {machineState(status).word.toLowerCase()}; its devices
            show once it is connected.
          </p>
        </SettingsSection>
      </SettingsPage>
    );
  }
  const devices = listed?.ok === true ? pairedDevices(listed.value) : null;
  return (
    <SettingsPage lead={LEAD}>
      <SettingsSection title={`Paired with ${label}`} action={picker}>
        {listed === null ? (
          <Skeleton />
        ) : !listed.ok ? (
          <Banner
            tone="error"
            action={
              <Button size="sm" onClick={load}>
                Try again
              </Button>
            }
          >
            The devices could not be listed: {listed.problem.message}{" "}
            {nextStep(listed.problem)}
          </Banner>
        ) : devices === null || devices.length === 0 ? (
          <p className="settings-empty">
            No devices yet. One paired with a link below shows up here.
          </p>
        ) : (
          <RowList label="Paired devices">
            {devices.map((device) => (
              <Row
                key={device.id}
                mark={{ icon: "devices" }}
                title={device.label}
                detail={{ facts: deviceLine(device) }}
                control={
                  <Button
                    variant="danger"
                    size="sm"
                    onClick={() => {
                      setRevoking(device);
                    }}
                  >
                    Revoke
                  </Button>
                }
              />
            ))}
          </RowList>
        )}
      </SettingsSection>
      <SettingsSection title="Pair another device">
        <InviteCard machine={machine} label={label} onInvited={load} />
      </SettingsSection>
      {revoking === null ? null : (
        <RevokeDialog
          device={revoking}
          label={label}
          onClose={() => {
            setRevoking(null);
          }}
          onRevoke={() => {
            const device = revoking;
            setRevoking(null);
            void revokeDevice(fleet, machine, device).then((outcome) => {
              if (outcome.ok) {
                toast(`Revoked ${device.label}`);
              } else {
                toast(
                  `${outcome.problem.message} ${nextStep(outcome.problem)}`,
                  "error",
                );
              }
              load();
            });
          }}
        />
      )}
    </SettingsPage>
  );
}
