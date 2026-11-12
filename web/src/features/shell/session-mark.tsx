// The mark a session's row leads with (DESIGN.md §2.2): a spinner while
// its agent works, a filled dot in the needs-you colour while it waits on
// its owner, a hollow dot while idle, a faint dash once it has ended.
// Decided by data/sidebar.ts.

import type { SessionMark as Mark } from "../../data/sidebar";
import { Spinner } from "../../ui/kit";

export function SessionMark({ mark }: { mark: Mark }) {
  switch (mark) {
    case "needs-you":
      return (
        <span className="sdot needs-you" role="img" aria-label="needs you" />
      );
    case "working":
      return (
        <span role="img" aria-label="working" className="sdot-spin">
          <Spinner size={12} />
        </span>
      );
    case "idle":
      return <span className="sdot" aria-hidden="true" />;
    case "ended":
      return <span className="sdot ended" role="img" aria-label="ended" />;
  }
}
