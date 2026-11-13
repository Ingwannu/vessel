// The composer's attach button (GOAT's: a 28px ghost "+" named "Attach
// files"; agent-orchestrator's "+" opens the same picker): the system's
// file picker, its files into the tray. Pasting or dropping files on the
// box does the same (prompt-box.tsx). Disabled, with why, until the
// machine has said its limits.

import type { Tray } from "../../data/tray";
import { IconButton, Tooltip, useFilePicker } from "../../ui/kit";
import type { Limits } from "./use-limits";

export function AttachButton({ tray, limits }: { tray: Tray; limits: Limits }) {
  const picker = useFilePicker((files) => {
    tray.add(files);
  });
  const why =
    limits.kind === "failed"
      ? `Files cannot be attached: ${limits.message}`
      : limits.kind === "none"
        ? "Reading what the machine takes…"
        : "Attach files";
  return (
    <>
      <Tooltip label={why}>
        <IconButton
          label="Attach files"
          icon="plus"
          shape="round"
          disabled={limits.kind !== "read"}
          onClick={picker.open}
        />
      </Tooltip>
      {picker.input}
    </>
  );
}
