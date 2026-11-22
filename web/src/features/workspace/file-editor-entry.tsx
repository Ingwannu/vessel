import { type ComponentProps, lazy, Suspense } from "react";

import { Skeleton } from "../../ui/kit";

const FileEditor = lazy(async () => ({
  default: (await import("./file-editor")).FileEditor,
}));

/** Defer the editing engine until the file enters edit mode. */
export function LazyFileEditor(props: ComponentProps<typeof FileEditor>) {
  return (
    <Suspense
      fallback={
        <div className="file-editor">
          <Skeleton />
        </div>
      }
    >
      <FileEditor {...props} />
    </Suspense>
  );
}
