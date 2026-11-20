//! The Files tab's viewers' words (F-F38, F-F40): Paseo's `panels.file`
//! and `message.diagram` strings (pinned 1c00b365c), the page's words
//! (`features/workspace/file-head.tsx`, `file-loading.tsx`,
//! `zoomable-image.tsx`) in English. See `text/mod.rs` for the convention.

use super::Lang;

impl Lang {
    /// The viewers' words in this language.
    #[must_use]
    pub const fn viewers(self) -> &'static ViewerWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's words.
#[derive(Debug)]
pub struct ViewerWords {
    /// The Preview|Source control's name for assistive technology.
    pub file_view: &'static str,
    /// The rendered side.
    pub preview: &'static str,
    /// The lines side.
    pub source: &'static str,
    /// An image being read.
    pub loading_file: &'static str,
    /// A file past the Preview's budget.
    pub too_large: &'static str,
    /// The image toolbar's buttons.
    pub zoom_in: &'static str,
    /// Zoom out.
    pub zoom_out: &'static str,
    /// Back to fit.
    pub reset_view: &'static str,
}

const EN: ViewerWords = ViewerWords {
    file_view: "File view",
    preview: "Preview",
    source: "Source",
    loading_file: "Loading file...",
    too_large: "This file is too large to display",
    zoom_in: "Zoom in",
    zoom_out: "Zoom out",
    reset_view: "Reset view",
};

const KO: ViewerWords = ViewerWords {
    file_view: "파일 보기",
    preview: "미리 보기",
    source: "소스",
    loading_file: "파일을 불러오는 중...",
    too_large: "파일이 너무 커서 표시할 수 없습니다",
    zoom_in: "확대",
    zoom_out: "축소",
    reset_view: "보기 초기화",
};
