//! About's official artwork attribution. Proper names and trusted links
//! live in `mark_attributions`; whole explanatory sentences are translated.
use super::Lang;

/// Words for the artwork attribution section in Settings › About.
#[derive(Debug)]
pub struct MarkText {
    /// Section heading; stays short.
    pub title: &'static str,
    /// Opens the vendor's source page; short button.
    pub source: &'static str,
    /// Opens the vendor's trademark terms; short button.
    pub terms: &'static str,
    /// Opens the redistributed original's software licence; short button.
    pub licence: &'static str,
    /// Complete section note, including the absence of endorsement.
    pub purpose: &'static str,
    lang: Lang,
}
impl Lang {
    /// Artwork attribution words in the chosen language.
    #[must_use]
    pub const fn marks(self) -> &'static MarkText {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}
const EN: MarkText = MarkText {
    title: "Agent artwork",
    source: "Source",
    terms: "Brand terms",
    licence: "Licence",
    purpose: "These marks identify agents. Their owners do not endorse Vessel.",
    lang: Lang::En,
};
const KO: MarkText = MarkText {
    title: "에이전트 로고",
    source: "출처",
    terms: "상표 사용 조건",
    licence: "라이선스",
    purpose: "에이전트를 식별하기 위한 로고입니다. 상표 소유자의 Vessel 보증을 뜻하지 않습니다.",
    lang: Lang::Ko,
};
impl MarkText {
    /// Attribution below an agent name; `owner` is the vendor's proper name.
    #[must_use]
    pub fn trademark(&self, owner: &str) -> String {
        match self.lang {
            Lang::En => format!("Trademark of {owner}."),
            Lang::Ko => format!("{owner}의 상표입니다."),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn attribution_is_a_complete_sentence_in_both_languages() {
        assert_eq!(Lang::En.marks().trademark("Google"), "Trademark of Google.");
        assert_eq!(Lang::Ko.marks().trademark("Google"), "Google의 상표입니다.");
        assert_eq!(
            Lang::En.marks().purpose,
            "These marks identify agents. Their owners do not endorse Vessel."
        );
        assert_eq!(
            Lang::Ko.marks().purpose,
            "에이전트를 식별하기 위한 로고입니다. 상표 소유자의 Vessel 보증을 뜻하지 않습니다."
        );
    }
}
