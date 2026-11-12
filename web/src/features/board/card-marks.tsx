// A card's marks beside its title (protocol 10): a pin when the session
// is pinned, and the sidebar's unread dot when its machine marks it
// unread, so the board and the sidebar say the same.

import { isPinned, isUnread } from "../../data/marks";
import { Icon } from "../../ui/kit";
import type { SessionInfo } from "../../wire/proto";

export function CardMarks({ session }: { session: SessionInfo }) {
  const pinned = isPinned(session);
  const unread = isUnread(session);
  if (!pinned && !unread) {
    return null;
  }
  return (
    <span className="card-marks">
      {pinned ? (
        <span className="card-pin" role="img" aria-label="pinned">
          <Icon name="pin" size={12} />
        </span>
      ) : null}
      {unread ? (
        <span className="sdot unread" role="img" aria-label="unread" />
      ) : null}
    </span>
  );
}
