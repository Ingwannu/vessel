// Settings (DESIGN.md §2.1): this client — the theme, the node book, and
// pairing this machine with a node from a link.

import { useState, type ReactElement } from "react";

import { useCrumbs, useFleet, useFleetState } from "../context";
import { nodeState, when } from "../data/format";
import { nodeLabel } from "../data/nodes";
import { chooseTheme, currentTheme, type Theme } from "../data/theme";
import {
  Badge,
  Button,
  EmptyState,
  Field,
  Segmented,
  TextField,
} from "../ui/kit";
import { toast } from "../ui/toasts";
import type { PairingOffer } from "../wire/proto";

export function SettingsPage(): ReactElement {
  useCrumbs([{ label: "Settings" }], "settings");
  const [theme, setTheme] = useState<Theme>(currentTheme);
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  return (
    <div className="page">
      <div className="page-head">
        <div>
          <p>
            This client: how it looks, and the nodes it knows. The book is the
            CLI&apos;s, shared with <code>vessel</code>.
          </p>
        </div>
      </div>
      <div className="page-body">
        <div className="panel">
          <h2>Theme</h2>
          <div className="pad">
            <Segmented
              label="Theme"
              value={theme}
              onChange={(value) => {
                chooseTheme(value);
                setTheme(value);
              }}
              options={[
                { value: "light", label: "Light", icon: "sun" },
                { value: "dark", label: "Dark", icon: "moon" },
              ]}
            />
          </div>
        </div>
        <div className="panel">
          <h2>Node book</h2>
          {book === null || book.length === 0 ? (
            <EmptyState title="No nodes yet">
              Add one with <code>vessel nodes add NAME ADDRESS</code>, or pair
              one below.
            </EmptyState>
          ) : (
            <table className="data book" aria-label="Node book">
              <thead>
                <tr>
                  <th scope="col">Name</th>
                  <th scope="col">Address</th>
                  <th scope="col">Transport</th>
                  <th scope="col">State</th>
                </tr>
              </thead>
              <tbody>
                {book.map((node) => {
                  const state = nodeState(statuses[node.key]);
                  return (
                    <tr key={node.key}>
                      <td className="name">
                        {nodeLabel(node, statuses[node.key])}
                        {node.default ? (
                          <Badge look="accent">default</Badge>
                        ) : null}
                      </td>
                      <td className="mono muted">{node.address}</td>
                      <td>
                        <Badge look="mono">{node.transport}</Badge>
                      </td>
                      <td>
                        <Badge tone={state.tone}>{state.word}</Badge>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </div>
        <PairWithNode />
      </div>
    </div>
  );
}

/** Pair this machine with a node from its link (as `vessel nodes add
 * NAME --pair`): the link is checked here first, and nothing is sent
 * until the operator has compared the fingerprint and confirmed. */
function PairWithNode() {
  const fleet = useFleet();
  const [link, setLink] = useState("");
  const [offer, setOffer] = useState<{
    offer: PairingOffer;
    fingerprint: string;
  } | null>(null);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inspect = () => {
    setError(null);
    fleet.bridge.op({ op: "inspect", link: link.trim() }).then(
      (result) => {
        if (result.op === "inspect") {
          setOffer({ offer: result.offer, fingerprint: result.fingerprint });
        }
      },
      (failure: unknown) => {
        setOffer(null);
        setError(failure instanceof Error ? failure.message : String(failure));
      },
    );
  };
  const pair = () => {
    setBusy(true);
    fleet.bridge.op({ op: "pair", name: name.trim(), link: link.trim() }).then(
      (result) => {
        setBusy(false);
        setOffer(null);
        setLink("");
        if (result.op === "pair") {
          toast(`Paired with ${result.node.label}`);
        }
        void fleet.loadBook();
      },
      (failure: unknown) => {
        setBusy(false);
        setError(failure instanceof Error ? failure.message : String(failure));
      },
    );
  };
  return (
    <div className="panel">
      <h2>Pair with a node</h2>
      <div className="pad">
        <p className="hint">
          Run <span className="mono">vessel node pair</span> on the node and
          paste its link here. The link is single use; it is sent nowhere until
          you confirm.
        </p>
        <div className="inline pair-new">
          <div className="grow">
            <Field label="Pairing link">
              <TextField
                mono
                placeholder="vessel://pair#…"
                value={link}
                onChange={(next) => {
                  setLink(next);
                  setOffer(null);
                }}
              />
            </Field>
          </div>
          <Button icon="link" disabled={link.trim() === ""} onClick={inspect}>
            Check
          </Button>
        </div>
        {offer === null ? null : (
          <div className="offer">
            <dl>
              <dt>node</dt>
              <dd>{offer.offer.node}</dd>
              <dt>fingerprint</dt>
              <dd>{offer.fingerprint}</dd>
              <dt>direct</dt>
              <dd>{offer.offer.direct.join(", ") || "none"}</dd>
              <dt>relays</dt>
              <dd>{offer.offer.relays.join(", ") || "none"}</dd>
              <dt>expires</dt>
              <dd>{when(offer.offer.expires)}</dd>
            </dl>
            <div className="inline pair-new">
              <Field label="Name in the book">
                <TextField value={name} onChange={setName} />
              </Field>
              <Button
                variant="primary"
                disabled={busy || name.trim() === ""}
                onClick={pair}
              >
                {busy ? "Pairing…" : "Pair"}
              </Button>
            </div>
          </div>
        )}
        {error === null ? null : (
          <p className="field-error" role="alert">
            {error}
          </p>
        )}
      </div>
    </div>
  );
}
