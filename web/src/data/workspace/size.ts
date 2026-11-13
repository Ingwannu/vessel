// A file's size as the page writes it: bytes up to a kibibyte, then KB
// and MB with one decimal under ten (binary units, as the node bounds
// are: 1 MiB a page, 256 KiB by default).

/** `bytes` in words: "812 B", "3.4 KB", "12 MB". */
export function sizeWord(bytes: number): string {
  if (bytes < 1024) {
    return `${String(bytes)} B`;
  }
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const shown = value < 10 ? value.toFixed(1) : String(Math.round(value));
  return `${shown} ${units[unit] ?? "GB"}`;
}
