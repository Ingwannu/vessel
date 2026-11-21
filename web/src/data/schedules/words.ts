// Paseo localizes the sidebar title and palette entry through the same key.
export function schedulesLabel(language: string): string {
  return /^ko(?:-|$)/i.test(language) ? "일정" : "Schedules";
}
