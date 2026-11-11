// The add-machine dialog's second stage, Paseo's "Connect to this host?":
// what the link says about the machine (read here, nothing sent), its
// key's fingerprint to compare with what that machine shows, and the
// name to give it, checked as typed.

import type { Offer } from "../../data/book";
import { expiresIn, offerRoutes } from "../../data/pairing";
import { Banner, Field, TextField } from "../../ui/kit";

export function AddConfirmStep({
  offer,
  name,
  onName,
  nameError,
  onSubmit,
}: {
  offer: Offer;
  name: string;
  onName: (name: string) => void;
  nameError: string | null;
  onSubmit: () => void;
}) {
  const expires = expiresIn(offer.offer.expires);
  const routes = offerRoutes(offer.offer);
  return (
    <>
      <dl className="add-offer" aria-label="The machine the link names">
        <div>
          <dt>Key</dt>
          <dd className="add-fingerprint">{offer.fingerprint}</dd>
        </div>
        <div>
          <dt>Reached</dt>
          <dd>
            {routes.length === 0 ? "No route in the link" : routes.join(" · ")}
          </dd>
        </div>
        <div>
          <dt>Machine id</dt>
          <dd className="mono">{offer.offer.node}</dd>
        </div>
        <div>
          <dt>Link expires</dt>
          <dd>{expires ?? "expired"}</dd>
        </div>
      </dl>
      <p className="add-step-hint">
        The key should match the one <code>vessel node status</code> shows on
        that machine. If it does not, do not connect.
      </p>
      {expires === null ? (
        <Banner tone="error">
          This link has expired. Run <code>vessel node pair</code> there again.
        </Banner>
      ) : null}
      <Field
        label="Name it"
        hint="How this page and the CLI will call it."
        error={name === "" ? null : nameError}
      >
        <TextField
          value={name}
          onChange={onName}
          placeholder="studio"
          spellCheck={false}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              onSubmit();
            }
          }}
        />
      </Field>
    </>
  );
}
