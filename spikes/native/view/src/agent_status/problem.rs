//! Translated operation failures are separate from retained diagnostics.

/// Meaning a person can act on; the link retains its original source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// Reconnect the selected machine.
    Disconnected,
    /// Wait for a request slot.
    Busy,
    /// No browser-to-node transport exists.
    NoTransport,
    /// Output pages could not be followed safely.
    InvalidOutput,
    /// The node rejected the call or its transport failed.
    Call,
}

impl Problem {
    /// Inline failure notice; busy and unsupported transports need attention.
    #[must_use]
    pub const fn variant(self) -> crate::alert::Variant {
        match self {
            Self::Busy | Self::NoTransport => crate::alert::Variant::Warning,
            Self::Disconnected | Self::InvalidOutput | Self::Call => crate::alert::Variant::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Problem;
    use crate::Lang;
    #[test]
    fn every_operation_failure_has_the_reference_notice_meaning() {
        use crate::alert::Variant;
        for (problem, expected) in [
            (Problem::Busy, Variant::Warning),
            (Problem::NoTransport, Variant::Warning),
            (Problem::Disconnected, Variant::Error),
            (Problem::InvalidOutput, Variant::Error),
            (Problem::Call, Variant::Error),
        ] {
            assert_eq!(problem.variant(), expected);
        }
    }
    #[test]
    fn each_failure_has_a_complete_sentence_in_both_languages() {
        for (problem, en, ko) in [
            (
                Problem::Disconnected,
                "The machine is not connected. Connect it, then try again.",
                "머신에 연결되지 않았습니다. 연결한 뒤 다시 시도하세요.",
            ),
            (
                Problem::Busy,
                "Settings is busy. Try again when the current operation finishes.",
                "설정 작업이 진행 중입니다. 현재 작업이 끝난 뒤 다시 시도하세요.",
            ),
            (
                Problem::NoTransport,
                "This browser cannot connect to a machine yet.",
                "이 브라우저에서는 아직 머신에 연결할 수 없습니다.",
            ),
            (
                Problem::InvalidOutput,
                "The machine returned inconsistent installation output. Refresh its connection before trying again.",
                "머신이 올바르지 않은 설치 출력을 보냈습니다. 연결을 새로 고친 뒤 다시 시도하세요.",
            ),
            (
                Problem::Call,
                "The machine could not complete this action. See its message below.",
                "머신에서 이 작업을 완료하지 못했습니다. 아래의 머신 메시지를 확인하세요.",
            ),
        ] {
            assert_eq!(Lang::En.agents().problem(problem), en);
            assert_eq!(Lang::Ko.agents().problem(problem), ko);
        }
    }
}
