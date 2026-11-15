//! Exhaustive access policy and expansion exit regressions.

use super::*;

#[test]
fn lock_policy_covers_every_submission_state() {
    for access in [Access::Editable, Access::ReadOnly] {
        for policy in [SubmitPolicy::Clear, SubmitPolicy::PreserveAndLock] {
            for submission in [
                Submission::Idle,
                Submission::Preparing,
                Submission::Sending,
                Submission::Accepted,
                Submission::Rejected,
            ] {
                let surface = Surface {
                    access,
                    submit_policy: policy,
                    submission,
                    ..Surface::default()
                };
                let locked = access == Access::ReadOnly
                    || (policy == SubmitPolicy::PreserveAndLock
                        && matches!(submission, Submission::Preparing | Submission::Sending));
                assert_eq!(surface.access() == Access::ReadOnly, locked);
            }
        }
    }
}

#[test]
fn expansion_requires_compact_editable_uncovered_and_three_rendered_lines() {
    for layout in [Layout::Desktop, Layout::Compact] {
        for access in [Access::Editable, Access::ReadOnly] {
            for overlay in [Overlay::None, Overlay::Dictation, Overlay::Voice] {
                for count in [
                    0,
                    FULLSCREEN_LINE_THRESHOLD,
                    FULLSCREEN_LINE_THRESHOLD.saturating_add(1),
                ] {
                    let mut surface = Surface {
                        access,
                        overlay,
                        ..Surface::default()
                    };
                    surface.expand(layout, RenderedLines::new(count));
                    let expected = layout == Layout::Compact
                        && access == Access::Editable
                        && overlay == Overlay::None
                        && count > FULLSCREEN_LINE_THRESHOLD;
                    assert_eq!(surface.extent == Extent::Fullscreen, expected);
                }
            }
        }
    }
}

#[test]
fn voice_readonly_escape_and_send_end_fullscreen_without_changing_mode() {
    for mode in [Mode::Chat, Mode::Terminal] {
        let expanded = Surface {
            mode,
            extent: Extent::Fullscreen,
            ..Surface::default()
        };
        for overlay in [Overlay::Dictation, Overlay::Voice] {
            let mut surface = expanded;
            surface.cover(overlay);
            assert_eq!(surface.extent, Extent::Inline);
            assert_eq!(surface.mode, mode);
        }
        let mut surface = expanded;
        surface.set_access(Access::ReadOnly);
        assert_eq!(surface.extent, Extent::Inline);
        let mut surface = expanded;
        surface.collapse();
        assert_eq!(surface.extent, Extent::Inline);
        assert_eq!(surface.mode, mode);
    }
}

#[test]
fn send_exits_fullscreen_before_preparing_and_failure_unlocks_without_reexpanding() {
    for policy in [SubmitPolicy::Clear, SubmitPolicy::PreserveAndLock] {
        let mut surface = Surface::new(Mode::Chat, Access::Editable, policy);
        surface.expand(
            Layout::Compact,
            RenderedLines::new(FULLSCREEN_LINE_THRESHOLD.saturating_add(1)),
        );
        assert_eq!(surface.extent(), Extent::Fullscreen);
        surface.set_submission(Submission::Preparing);
        assert_eq!(surface.extent(), Extent::Inline);
        assert_eq!(
            surface.access() == Access::ReadOnly,
            policy == SubmitPolicy::PreserveAndLock
        );
        surface.set_submission(Submission::Rejected);
        assert_eq!(surface.access(), Access::Editable);
        assert_eq!(surface.extent(), Extent::Inline);
        assert_eq!(surface.mode(), Mode::Chat);
    }
}
