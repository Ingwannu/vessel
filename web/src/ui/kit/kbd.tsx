// Kbd (DESIGN.md §6.13): a shortcut. GOAT's key chip (20px, a 7% wash,
// 12px medium) where a shortcut stands beside a command; `text` is the
// reference's quieter form, the keys written as text, where the chip
// would be the loudest thing in a row (the sidebar's search).

import "./kbd.css";

export function Kbd({
  keys,
  look = "key",
}: {
  /** The keys, one string each: `["⌘", "K"]`. */
  keys: readonly string[];
  look?: "key" | "text";
}) {
  return (
    <kbd className={look === "key" ? "kbd key" : "kbd"}>{keys.join("")}</kbd>
  );
}
