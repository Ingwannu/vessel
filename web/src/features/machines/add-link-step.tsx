// The add-machine dialog's first stage: what to run on the other machine
// (the real command, to copy), then where to paste what it prints.
// Nothing is sent anywhere until the person has seen the machine's key
// on the next stage.

import { PAIR_COMMAND, START_COMMAND } from "../../data/pairing";
import { CommandLine, Field, TextField } from "../../ui/kit";

export function AddLinkStep({
  link,
  onLink,
  error,
  onSubmit,
}: {
  link: string;
  onLink: (link: string) => void;
  /** Why the pasted text was not read as a link. */
  error: string | null;
  onSubmit: () => void;
}) {
  return (
    <ol className="add-steps">
      <li>
        <span className="add-step-number" aria-hidden="true">
          1
        </span>
        <div className="add-step-body">
          <p className="add-step-title">On the other machine, run</p>
          <CommandLine command={PAIR_COMMAND} />
          <p className="add-step-hint">
            Vessel must be running there (<code>{START_COMMAND}</code>). It
            prints a link that works once, for ten minutes.
          </p>
        </div>
      </li>
      <li>
        <span className="add-step-number" aria-hidden="true">
          2
        </span>
        <div className="add-step-body">
          <Field
            label="Paste the link it printed"
            hint="Nothing is sent until you have checked the machine's key."
            error={error}
          >
            <TextField
              mono
              value={link}
              onChange={onLink}
              placeholder="vessel://pair#…"
              spellCheck={false}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  onSubmit();
                }
              }}
            />
          </Field>
        </div>
      </li>
    </ol>
  );
}
