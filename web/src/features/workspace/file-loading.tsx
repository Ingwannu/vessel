// Paseo's file panel loading state (`file-panels/loading.tsx`): a spinner
// and "Loading file..." in the panel's middle.

import "./file-viewers.css";

import { Spinner } from "../../ui/kit";

export function FileLoading() {
  return (
    <div className="file-state" data-testid="file-preview-loading">
      <Spinner size={16} />
      <span>Loading file...</span>
    </div>
  );
}
