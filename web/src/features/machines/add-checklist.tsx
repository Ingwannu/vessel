// The add-machine dialog's last stage: GOAT's preflight checklist. A
// vertical list, each row an icon, a name and one line of result; a
// pending row spins, a failed one says what to do and offers the command
// for it. The rows are `pairingChecks`' decision.

import type { Check, CheckState } from "../../data/pairing";
import { CommandLine, Icon, type IconName, Spinner } from "../../ui/kit";

function mark(state: CheckState): { icon: IconName | null; word: string } {
  switch (state) {
    case "pending":
      return { icon: null, word: "Checking" };
    case "passed":
      return { icon: "circle-check", word: "Passed" };
    case "failed":
      return { icon: "circle-x", word: "Failed" };
    case "skipped":
      return { icon: "circle-dashed", word: "Skipped" };
  }
}

export function AddChecklist({ checks }: { checks: readonly Check[] }) {
  return (
    <ol className="add-checks" aria-label="Adding the machine">
      {checks.map((check) => {
        const shown = mark(check.state);
        return (
          <li key={check.id} className={`add-check ${check.state}`}>
            <span className="add-check-mark">
              {shown.icon === null ? (
                <Spinner size={14} />
              ) : (
                <Icon name={shown.icon} size={16} />
              )}
              <span className="visually-hidden">{shown.word}:</span>
            </span>
            <div className="add-check-body">
              <p className="add-check-name">{check.label}</p>
              <p className="add-check-result" aria-live="polite">
                {check.result}
              </p>
              {check.fix === undefined ? null : (
                <>
                  <p className="add-check-fix">{check.fix.text}</p>
                  {check.fix.command === undefined ? null : (
                    <CommandLine command={check.fix.command} />
                  )}
                </>
              )}
            </div>
          </li>
        );
      })}
    </ol>
  );
}
