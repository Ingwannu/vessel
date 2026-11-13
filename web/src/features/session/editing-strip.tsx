// Over the composer's text while a queued prompt is open in it
// (agent-orchestrator's "Editing queued message" with "Cancel edit"):
// the box holds the queued words, not the draft, until it is saved with
// the update button or closed here or with Escape.

import { Button } from "../../ui/kit";

export function EditingStrip({ onCancel }: { onCancel: () => void }) {
  return (
    <div className="editing-strip" role="status">
      <span>Editing queued message</span>
      <Button size="xs" variant="ghost" onClick={onCancel}>
        Cancel edit
      </Button>
    </div>
  );
}
