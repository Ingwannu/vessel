// Skeleton (DESIGN.md §6.11): three rows standing in for a list that is
// loading, announced as loading.

import "./skeleton.css";

export function Skeleton() {
  return (
    <div className="skeleton" aria-label="Loading" role="status">
      <div className="skel" />
      <div className="skel" />
      <div className="skel" />
    </div>
  );
}
