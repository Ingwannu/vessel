// A part the screens share that needs the page's own services (the
// toasts), so it cannot live in the kit: the copy button. The controls
// and the marks are the kit's (ui/kit, ADR 0021).

import { IconButton, Tooltip } from "./kit";
import { toast } from "./toasts";

/** A copy-to-clipboard button, named by its tooltip. */
export function CopyButton({
  text,
  label,
  onCopied,
}: {
  text: string;
  label: string;
  onCopied?: () => void;
}) {
  return (
    <Tooltip label={label}>
      <IconButton
        label={label}
        icon="copy"
        size="sm"
        onClick={() => {
          navigator.clipboard.writeText(text).then(onCopied, () => {
            toast("The clipboard refused the text", "error");
          });
        }}
      />
    </Tooltip>
  );
}
