//! Translated operation failures are separate from retained diagnostics.

/// Meaning a person can act on; the link retains its original source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// Refresh and confirm against the new physical node.
    NodeChanged,
    /// The peer has no provider administration method.
    OlderNode,
    /// The node refused administrative access.
    Permission,
    /// An account profile still extends this provider.
    ProviderInUse,
    /// Read the provider list before making another removal decision.
    RemovalUnknown,
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
            Self::Busy
            | Self::NoTransport
            | Self::NodeChanged
            | Self::OlderNode
            | Self::Permission
            | Self::ProviderInUse
            | Self::RemovalUnknown => crate::alert::Variant::Warning,
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
            (Problem::NodeChanged, Variant::Warning),
            (Problem::OlderNode, Variant::Warning),
            (Problem::Permission, Variant::Warning),
            (Problem::ProviderInUse, Variant::Warning),
            (Problem::RemovalUnknown, Variant::Warning),
        ] {
            assert_eq!(problem.variant(), expected);
        }
    }
    #[test]
    fn each_failure_has_a_complete_sentence_in_both_languages() {
        for (problem, en, ko) in [
            (
                Problem::NodeChanged,
                "The machine behind this name changed. Select “Refresh” to read its providers, then confirm again.",
                "이 이름이 가리키는 머신이 변경되었습니다. “새로 고침”을 선택해 프로바이더 목록을 읽은 뒤 다시 확인하세요.",
            ),
            (
                Problem::OlderNode,
                "Update Vessel on this machine to remove providers.",
                "프로바이더를 제거하려면 이 머신의 Vessel을 업데이트하세요.",
            ),
            (
                Problem::Permission,
                "This device cannot remove providers on this machine. Ask the machine’s administrator to grant access.",
                "이 기기에는 이 머신의 프로바이더를 제거할 권한이 없습니다. 머신 관리자에게 접근 권한을 요청하세요.",
            ),
            (
                Problem::ProviderInUse,
                "A profile uses this provider. Change or remove that profile on the machine before removing the provider.",
                "프로필에서 이 프로바이더를 사용 중입니다. 머신에서 해당 프로필을 변경하거나 제거한 뒤 프로바이더를 제거하세요.",
            ),
            (
                Problem::RemovalUnknown,
                "The removal result is not known. Select “Refresh” to read the provider list before trying again.",
                "제거 결과를 확인하지 못했습니다. 다시 시도하기 전에 “새로 고침”을 선택해 프로바이더 목록을 읽으세요.",
            ),
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
                "The machine could not complete this action.",
                "머신에서 이 작업을 완료하지 못했습니다.",
            ),
        ] {
            assert_eq!(Lang::En.agents().problem(problem), en);
            assert_eq!(Lang::Ko.agents().problem(problem), ko);
        }
    }
}
