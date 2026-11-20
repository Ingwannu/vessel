// Pairing a new device with a machine (Settings › Devices): a name, New
// link, then the link and its QR code, hidden until asked for.
//
// The link is a credential until a device uses it, so it is shown only
// on request and hidden again after 30 seconds, when the window loses
// focus or the tab is hidden, and once it is copied (as the old Devices
// page did; ADR 0010). It is never put in a toast or an address.

import "./invite-card.css";

import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import { inviteDevice, type Pairing } from "../../data/devices";
import { when } from "../../data/format";
import { withNextStep } from "../../data/outcome";
import type { NodeKey } from "../../wire/bridge";
import { Button, Field, Icon, TextField } from "../../ui/kit";
import { CopyButton } from "../../ui/parts";
import { toast } from "../../ui/toasts";

/** How long a shown link stays on screen. */
const SHOWN_MS = 30_000;

export function InviteCard({
  machine,
  label,
  onInvited,
}: {
  machine: NodeKey;
  /** The machine's name. */
  label: string;
  /** A device may have paired with the previous link meanwhile. */
  onInvited: () => void;
}) {
  const fleet = useFleet();
  const [name, setName] = useState("phone");
  const [busy, setBusy] = useState(false);
  const [pairing, setPairing] = useState<Pairing | null>(null);
  const [showing, setShowing] = useState(false);
  useEffect(() => {
    if (!showing) {
      return undefined;
    }
    const hide = () => {
      setShowing(false);
    };
    const timer = setTimeout(hide, SHOWN_MS);
    window.addEventListener("blur", hide);
    document.addEventListener("visibilitychange", hide);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("blur", hide);
      document.removeEventListener("visibilitychange", hide);
    };
  }, [showing]);
  const invite = () => {
    setBusy(true);
    void inviteDevice(fleet, machine, name).then((outcome) => {
      setBusy(false);
      onInvited();
      if (outcome.ok) {
        setPairing(outcome.value);
        setShowing(false);
      } else {
        toast(withNextStep(outcome.problem), "error");
      }
    });
  };
  return (
    <div className="invite-card">
      <div className="invite-form">
        <div className="invite-name">
          <Field
            label="Device name"
            hint={`A link that pairs one device with ${label}; it works once, for ten minutes.`}
          >
            <TextField value={name} onChange={setName} size="sm" />
          </Field>
        </div>
        <Button variant="primary" size="sm" busy={busy} onClick={invite}>
          New link
        </Button>
      </div>
      {pairing === null ? null : (
        <div className="invite-link">
          <div className="invite-qr">
            {showing ? (
              <img
                src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(pairing.svg)}`}
                alt="Pairing link as a QR code"
                width={120}
                height={120}
              />
            ) : (
              <Icon name="qr" size={20} />
            )}
          </div>
          <div className="invite-text">
            <p className="invite-hint">
              Scan it on the device, or paste the link at its prompt, never on a
              command line. The device should show this key:{" "}
              <span className="mono">{pairing.fingerprint}</span>. Expires{" "}
              {when(pairing.invitation.expires)}.
            </p>
            {showing ? (
              <div className="invite-box">
                <span className="invite-box-text">
                  {pairing.invitation.link}
                </span>
                <CopyButton
                  text={pairing.invitation.link}
                  label="Copy pairing link"
                  onCopied={() => {
                    setShowing(false);
                  }}
                />
              </div>
            ) : null}
            <div>
              <Button
                size="sm"
                onClick={() => {
                  setShowing(!showing);
                }}
              >
                {showing ? "Hide pairing link" : "Show pairing link"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
