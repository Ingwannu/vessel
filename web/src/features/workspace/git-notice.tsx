// A git refusal, said plainly where the action was asked (the Changes
// tab's head, a dialog's body): what happened, what to do, the paths it
// names (the first few, then how many more) and any output that came with
// it (a hook's, the remote's). It stays until dismissed or the next action.

import type { RefusalWords } from "../../data/workspace/git-words";
import { Banner, Button } from "../../ui/kit";

/** How many paths are listed before "and N more". */
const SHOWN_PATHS = 6;

export function GitNotice({
  notice,
  onDismiss,
}: {
  notice: RefusalWords;
  onDismiss?: () => void;
}) {
  const shown = notice.paths.slice(0, SHOWN_PATHS);
  const rest = notice.paths.length - shown.length;
  return (
    <div className="git-notice">
      <Banner
        tone="error"
        {...(onDismiss === undefined
          ? {}
          : {
              action: (
                <Button size="xs" variant="ghost" onClick={onDismiss}>
                  Dismiss
                </Button>
              ),
            })}
      >
        <strong>{notice.title}</strong> {notice.next}
        {shown.length === 0 ? null : (
          <ul className="git-notice-paths">
            {shown.map((path) => (
              <li key={path}>{path}</li>
            ))}
            {rest > 0 ? <li>and {String(rest)} more</li> : null}
          </ul>
        )}
        {notice.output === null || notice.output === "" ? null : (
          <pre className="git-notice-output">{notice.output}</pre>
        )}
      </Banner>
    </div>
  );
}
