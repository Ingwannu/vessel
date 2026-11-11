// Avatar (DESIGN.md §6.12): an agent's mark, a letter in a faint 18px
// tile (agent-orchestrator's agent avatar, with a letter where it has the
// vendor's logo: the logos are theirs). On a board card and in a
// session's title bar.

import "./avatar.css";

export function Avatar({ letter, label }: { letter: string; label: string }) {
  return (
    <span className="avatar" role="img" aria-label={label}>
      {letter}
    </span>
  );
}
