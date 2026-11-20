// The sidebar's Show choices and the composer's diff-pill name.
// Paseo `i18n/resources/en.ts` 1199–1209 and 941, `ko.ts` 1198–1208 and 940.
// The browser language, as the workspace tab chrome does.

export interface SidebarWords {
  show: string;
  diffStats: string;
  activity: string;
  openChanges: string;
}

const en: SidebarWords = {
  show: "Show",
  diffStats: "Diff stats",
  activity: "Last activity",
  openChanges: "Open Changes tab",
};

const ko: SidebarWords = {
  show: "표시 항목",
  diffStats: "Diff 통계",
  activity: "마지막 활동",
  openChanges: "변경사항 탭 열기",
};

export function sidebarWords(language: string): SidebarWords {
  return /^ko(?:-|$)/i.test(language) ? ko : en;
}
