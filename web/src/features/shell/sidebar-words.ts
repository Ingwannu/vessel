// The composer's diff-pill name. Paseo `i18n/resources/en.ts` 941,
// `ko.ts` 940. The browser language, as the workspace tab chrome does.
// The Display menu does not use these words: Grouping, Sort, and
// "Show ended sessions" are English literals, so its Show group is too.

export interface SidebarWords {
  openChanges: string;
}

const en: SidebarWords = {
  openChanges: "Open Changes tab",
};

const ko: SidebarWords = {
  openChanges: "변경사항 탭 열기",
};

export function sidebarWords(language: string): SidebarWords {
  return /^ko(?:-|$)/i.test(language) ? ko : en;
}
