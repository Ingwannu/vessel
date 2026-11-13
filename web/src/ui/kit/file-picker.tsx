// The system's file picker, opened from a kit button (milestone 11's
// attach): a hidden file input the kit owns, so no screen draws a native
// control (`npm run lint`'s rule), and a function that opens it.

import { type ReactNode, useRef } from "react";

/** A hidden picker for several files, and the function that opens it;
 * render `input` once beside the button. */
export function useFilePicker(onFiles: (files: File[]) => void): {
  open: () => void;
  input: ReactNode;
} {
  const ref = useRef<HTMLInputElement>(null);
  return {
    open: () => {
      ref.current?.click();
    },
    input: (
      <input
        ref={ref}
        type="file"
        multiple
        hidden
        tabIndex={-1}
        aria-hidden="true"
        onChange={(event) => {
          const files = Array.from(event.target.files ?? []);
          // The same file picked again must fire a change next time.
          event.target.value = "";
          if (files.length > 0) {
            onFiles(files);
          }
        }}
      />
    ),
  };
}
