// Devices (DESIGN.md §2.1): the clients paired with the selected node,
// pairing a new one with a link and its QR code, and revoking one.

import { useCallback, useEffect, useState, type ReactElement } from "react";

import { useCrumbs, useFleet, useFleetState } from "../context";
import { ago, when } from "../data/format";
import { useWhere } from "../data/where";
import { labelOf, selectedNode } from "../data/nodes";
import {
  Badge,
  Button,
  Dialog,
  EmptyState,
  Field,
  Icon,
  Skeleton,
  TextField,
} from "../ui/kit";
import { CopyButton } from "../ui/parts";
import { toast } from "../ui/toasts";
import type { DeviceInfo, Invitation } from "../wire/proto";

/** Ten minutes, as the CLI's `vessel node pair`. */
const LINK_TTL_SECS = 600;

interface Pairing {
  invitation: Invitation;
  svg: string;
  fingerprint: string;
}

export function DevicesPage(): ReactElement {
  const where = useWhere();
  const fleet = useFleet();
  const node = useFleetState((state) => selectedNode(state, where.node));
  const label = useFleetState((state) =>
    node === null ? "" : labelOf(state, node),
  );
  const connected = useFleetState((state) =>
    node === null ? false : state.statuses[node]?.state === "connected",
  );
  // Kept with the node they are for, so switching nodes shows nothing
  // stale and needs no reset.
  const [listed, setListed] = useState<{
    node: string;
    devices: readonly DeviceInfo[] | null;
    error: string | null;
  } | null>(null);
  const [paired, setPairing] = useState<(Pairing & { node: string }) | null>(
    null,
  );
  const devices = listed?.node === node ? listed.devices : null;
  const error = listed?.node === node ? listed.error : null;
  const pairing = paired?.node === node ? paired : null;
  const [revealed, setRevealed] = useState<Pairing | null>(null);
  const showing = pairing !== null && revealed === pairing;
  useEffect(() => {
    if (!showing) return;
    const hide = () => {
      setRevealed(null);
    };
    const timer = setTimeout(hide, 30_000);
    window.addEventListener("blur", hide);
    document.addEventListener("visibilitychange", hide);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("blur", hide);
      document.removeEventListener("visibilitychange", hide);
    };
  }, [showing]);
  const [name, setName] = useState("phone");
  const [revoking, setRevoking] = useState<DeviceInfo | null>(null);
  useCrumbs(
    [{ label: "Devices" }, ...(node === null ? [] : [{ label }])],
    `${node ?? ""}${label}`,
  );

  const load = useCallback(() => {
    if (node === null) {
      return;
    }
    fleet.bridge.call(node, "device.list", null).then(
      (list) => {
        setListed({ node, devices: list, error: null });
      },
      (failure: unknown) => {
        setListed({
          node,
          devices: null,
          error: failure instanceof Error ? failure.message : String(failure),
        });
      },
    );
  }, [fleet, node]);
  useEffect(() => {
    if (connected) {
      load();
    }
  }, [load, connected]);

  const invite = async () => {
    if (node === null) {
      return;
    }
    try {
      const invitation = await fleet.bridge.call(node, "device.invite", {
        label: name.trim() === "" ? null : name.trim(),
        ttl_secs: LINK_TTL_SECS,
      });
      const [qr, offer] = await Promise.all([
        fleet.bridge.op({ op: "qr", text: invitation.link }),
        fleet.bridge.op({ op: "inspect", link: invitation.link }),
      ]);
      // A device may have paired with the previous link meanwhile.
      load();
      setPairing({
        node,
        invitation,
        svg: qr.op === "qr" ? qr.svg : "",
        fingerprint: offer.op === "inspect" ? offer.fingerprint : "",
      });
    } catch (failure) {
      toast(
        failure instanceof Error ? failure.message : String(failure),
        "error",
      );
    }
  };

  const revoke = (device: DeviceInfo) => {
    if (node === null) {
      return;
    }
    fleet.bridge.call(node, "device.revoke", { device: device.id }).then(
      () => {
        toast(`Revoked ${device.label}`);
        load();
      },
      (failure: unknown) => {
        toast(
          failure instanceof Error ? failure.message : String(failure),
          "error",
        );
      },
    );
  };

  const live =
    devices?.filter(
      (device) => device.revoked_at === undefined || device.revoked_at === null,
    ) ?? [];
  return (
    <div className="page">
      <div className="page-head">
        <div>
          <p>
            Paired clients for this node. Revoking closes their open
            connections. Pairing links expire in 10 minutes and are single-use.
          </p>
        </div>
      </div>
      <div className="page-body">
        {node === null || !connected ? (
          <EmptyState
            title={
              node === null ? "No node selected" : `${label} is not connected`
            }
          >
            Devices are per node: pick a connected one in the sidebar.
          </EmptyState>
        ) : (
          <>
            <div className="panel">
              <h2>Pair a new device</h2>
              <div className="pad pair">
                <div
                  className="qr"
                  aria-label={
                    !showing ? undefined : "Pairing link as a QR code"
                  }
                >
                  {!showing ? (
                    <Icon name="qr" size={20} />
                  ) : (
                    <img
                      src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(pairing.svg)}`}
                      alt="Pairing link as a QR code"
                      width={160}
                      height={160}
                    />
                  )}
                </div>
                <div>
                  <p className="hint">
                    Show this on {label}. Paste the link into the other device
                    at its prompt, never on a command line. Compare the
                    fingerprint with{" "}
                    <span className="mono">vessel node status</span> before
                    confirming there.
                  </p>
                  {!showing ? null : (
                    <>
                      <div className="link-box">
                        <Icon name="link" size={14} />
                        <span className="link-text">
                          {pairing.invitation.link}
                        </span>
                        <CopyButton
                          text={pairing.invitation.link}
                          label="Copy pairing link"
                          onCopied={() => {
                            setRevealed(null);
                          }}
                        />
                      </div>
                      <p className="hint">
                        Fingerprint{" "}
                        <span className="mono">{pairing.fingerprint}</span> ·
                        expires {when(pairing.invitation.expires)} · single use
                      </p>
                    </>
                  )}
                  {pairing === null ? null : (
                    <Button
                      onClick={() => {
                        setRevealed(showing ? null : pairing);
                      }}
                    >
                      {showing ? "Hide pairing link" : "Show pairing link"}
                    </Button>
                  )}
                  <div className="inline pair-new">
                    <Field label="Label">
                      <TextField value={name} onChange={setName} />
                    </Field>
                    <Button variant="primary" onClick={() => void invite()}>
                      New link
                    </Button>
                  </div>
                </div>
              </div>
            </div>
            {error !== null ? (
              <EmptyState title={`Devices on ${label} cannot be listed`}>
                {error}
              </EmptyState>
            ) : devices === null ? (
              <Skeleton />
            ) : live.length === 0 ? (
              <EmptyState title={`No devices paired with ${label}`}>
                A device paired with a link above shows up here.
              </EmptyState>
            ) : (
              <div className="table-wrap">
                <table className="data" aria-label="Paired devices">
                  <thead>
                    <tr>
                      <th scope="col">Device</th>
                      <th scope="col">Scopes</th>
                      <th scope="col">Last seen</th>
                      <th scope="col">Created</th>
                      <th scope="col">
                        <span className="visually-hidden">Actions</span>
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {live.map((device) => (
                      <tr key={device.id}>
                        <td>
                          <div className="stack">
                            <span className="name">{device.label}</span>
                            <span className="mono muted">
                              {device.id.slice(0, 8)}
                            </span>
                          </div>
                        </td>
                        <td>
                          {device.scopes.map((scope) => (
                            <Badge key={scope}>{scope}</Badge>
                          ))}
                        </td>
                        <td className="mono muted">
                          {device.last_seen === undefined ||
                          device.last_seen === null
                            ? "never"
                            : ago(device.last_seen)}
                        </td>
                        <td className="mono muted">{ago(device.created_at)}</td>
                        <td>
                          <div className="row-actions">
                            <Button
                              variant="danger"
                              size="sm"
                              onClick={() => {
                                setRevoking(device);
                              }}
                            >
                              Revoke
                            </Button>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </>
        )}
      </div>
      {revoking === null ? null : (
        <Dialog
          title={`Revoke ${revoking.label}?`}
          onClose={() => {
            setRevoking(null);
          }}
          footer={
            <>
              <Button
                variant="ghost"
                onClick={() => {
                  setRevoking(null);
                }}
              >
                Cancel
              </Button>
              <Button
                variant="danger-solid"
                onClick={() => {
                  revoke(revoking);
                  setRevoking(null);
                }}
              >
                Revoke
              </Button>
            </>
          }
        >
          <p className="hint">
            {revoking.label} loses access to {label} now; its open connections
            close. Pairing again needs a new link.
          </p>
        </Dialog>
      )}
    </div>
  );
}
