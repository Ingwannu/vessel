// A file's editor in the Files tab: its whole text in a bare monospace
// area; ⌘/Ctrl+S saves. The file view keeps the edit and its save.

import { TextArea } from "../../ui/kit";
import type { RelPath } from "../../wire/proto";

export function FileEditor({
  path,
  draft,
  saving,
  onChange,
  onSave,
}: {
  path: RelPath;
  draft: string;
  saving: boolean;
  onChange: (draft: string) => void;
  onSave: () => void;
}) {
  return (
    <div
      className="file-editor"
      onKeyDown={(event) => {
        if (
          (event.metaKey || event.ctrlKey) &&
          event.key.toLowerCase() === "s"
        ) {
          event.preventDefault();
          onSave();
        }
      }}
    >
      <TextArea
        look="bare"
        mono
        fill
        value={draft}
        onChange={onChange}
        readOnly={saving}
        aria-label={`Edit ${path}`}
      />
    </div>
  );
}
