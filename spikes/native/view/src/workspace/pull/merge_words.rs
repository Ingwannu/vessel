//! The merge's words: Paseo's labels and its "Merge PR isn't available"
//! sentences (`workspace.git.actions.mergePr`, `unavailable.mergePr*`),
//! chosen from the node's closed refusal; the forge's own message is the
//! alert's detail, never part of the sentence.

use std::borrow::Cow;

use vessel_proto::forge::{ForgeAccess, MergeMethod, MergeRefusal, PullState};

use crate::localization::Translator;

/// A merge row's label (Paseo's "Merge PR (squash)" and its kin).
#[must_use]
pub fn merge_label(method: MergeMethod, catalog: &Translator) -> &str {
    let words = catalog.pulls();
    match method {
        MergeMethod::Squash => words.merge_squash(),
        MergeMethod::Merge => words.merge_merge(),
        MergeMethod::Rebase => words.merge_rebase(),
    }
}

/// Why `pr.merge` merged nothing, whole.
#[must_use]
pub fn merge_refusal_words<'a>(refusal: &MergeRefusal, catalog: &'a Translator) -> Cow<'a, str> {
    let words = catalog.pulls();
    let text = match refusal {
        MergeRefusal::NoPullRequest => words.merge_missing(),
        MergeRefusal::OtherPullRequest { number, .. } => {
            return words
                .merge_other(&number.get().to_string())
                .map_or(Cow::Borrowed(words.merge_refused_unknown()), Cow::Owned);
        }
        MergeRefusal::NotOpen {
            state: PullState::Merged,
            ..
        } => words.merge_already(),
        MergeRefusal::NotOpen { .. } => words.merge_closed(),
        MergeRefusal::Draft => words.merge_draft(),
        MergeRefusal::HeadMoved { .. } => words.merge_moved(),
        MergeRefusal::NotMergeable { .. } => words.merge_not_ready(),
        MergeRefusal::MethodNotAllowed => words.merge_method(),
        MergeRefusal::MergeQueue => words.merge_queue(),
        MergeRefusal::CannotPin => words.merge_pin(),
        MergeRefusal::Access { access, .. } => access_words(access, catalog),
        MergeRefusal::Rejected { .. } => words.merge_rejected(),
        MergeRefusal::Unknown => words.merge_refused_unknown(),
    };
    Cow::Borrowed(text)
}

/// Paseo says "GitHub isn't connected" for every reason the node could not
/// reach the forge; the rest are the merge's general failure.
fn access_words<'a>(access: &ForgeAccess, catalog: &'a Translator) -> &'a str {
    let words = catalog.pulls();
    match access {
        ForgeAccess::SignedOut { .. }
        | ForgeAccess::NotSupported { .. }
        | ForgeAccess::Refused { .. }
        | ForgeAccess::RateLimited { .. }
        | ForgeAccess::Unreachable { .. } => words.merge_no_github(),
        ForgeAccess::Ready { .. }
        | ForgeAccess::NotPlaced
        | ForgeAccess::NoBranch
        | ForgeAccess::OnBase
        | ForgeAccess::NoRemote
        | ForgeAccess::Limit
        | ForgeAccess::Ended
        | ForgeAccess::Unknown => words.merge_refused_unknown(),
    }
}

#[cfg(test)]
mod tests {
    //! Paseo's sentences kept; a forge's message never in them.

    use vessel_proto::forge::{GhState, MergeMethod, MergeRefusal, PullState};

    use super::{merge_label, merge_refusal_words};
    use crate::localization::{ENGLISH, Locale, Translator};

    fn en() -> &'static Translator {
        Translator::get(ENGLISH).unwrap()
    }

    fn ko() -> &'static Translator {
        Translator::get(Locale::from_tag("ko").unwrap()).unwrap()
    }

    #[test]
    fn the_labels_are_paseo_s() {
        assert_eq!(merge_label(MergeMethod::Squash, en()), "Merge PR (squash)");
        assert_eq!(merge_label(MergeMethod::Rebase, ko()), "PR 병합(리베이스)");
    }

    #[test]
    fn each_refusal_says_why_and_keeps_the_forge_s_words_out() {
        assert_eq!(
            merge_refusal_words(&MergeRefusal::Draft, en()),
            "Merge PR isn't available because the pull request is still a draft"
        );
        assert_eq!(
            merge_refusal_words(
                &MergeRefusal::NotOpen {
                    state: PullState::Merged
                },
                ko()
            ),
            "풀 리퀘스트가 이미 병합되어 PR 병합을 사용할 수 없습니다"
        );
        let rejected = merge_refusal_words(
            &MergeRefusal::Rejected {
                message: "Required status check is expected".to_owned(),
            },
            en(),
        );
        assert_eq!(rejected, "The merge was refused");
        let other = merge_refusal_words(
            &MergeRefusal::OtherPullRequest {
                number: vessel_proto::forge::PullNumber::new(43).unwrap(),
            },
            en(),
        );
        assert!(
            other
                .replace(['\u{2068}', '\u{2069}'], "")
                .ends_with("is now #43")
        );
        let signed_out = MergeRefusal::Access {
            access: vessel_proto::forge::ForgeAccess::SignedOut {
                host: "github.com".parse().unwrap(),
                gh: GhState::SignedOut,
            },
        };
        assert!(merge_refusal_words(&signed_out, en()).contains("GitHub isn't connected"));
    }
}
