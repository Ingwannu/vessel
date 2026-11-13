//! Complete error sentences for machine-book actions; raw errors are not text.
use super::Lang;
use crate::machine_book::Failure;

impl Lang {
    /// Inline machine-list read/edit failure; file faults may include their safe
    /// path, line, column and known field. Allow wrapping.
    #[must_use]
    pub fn machine_book_failure(self, failure: &Failure) -> String {
        let text = match (self, failure) {
            (_, Failure::File(fault)) => return self.machines_unreadable(fault),
            (Self::En, Failure::NoConfiguration) => {
                "The configuration folder could not be found. Choose a configuration folder and restart Vessel."
            }
            (Self::Ko, Failure::NoConfiguration) => {
                "설정 폴더를 찾을 수 없습니다. 설정 폴더를 지정하고 Vessel을 다시 시작하세요."
            }
            (Self::En, Failure::Read) => {
                "The machine list could not be read. Check that your account can read the configuration folder, then try again."
            }
            (Self::Ko, Failure::Read) => {
                "머신 목록을 읽을 수 없습니다. 현재 계정의 설정 폴더 읽기 권한을 확인하고 다시 시도하세요."
            }
            (Self::En, Failure::InvalidBook) => {
                "The machine list has an invalid entry. Correct the machine list file, then try again."
            }
            (Self::Ko, Failure::InvalidBook) => {
                "머신 목록에 잘못된 항목이 있습니다. 머신 목록 파일을 고치고 다시 시도하세요."
            }
            (Self::En, Failure::Missing) => {
                "This machine is no longer in the list. Refresh the list and choose another machine."
            }
            (Self::Ko, Failure::Missing) => {
                "이 머신은 더 이상 목록에 없습니다. 목록을 새로 고치고 다른 머신을 선택하세요."
            }
            (Self::En, Failure::Taken) => {
                "Another machine already has that name. Choose a different name."
            }
            (Self::Ko, Failure::Taken) => {
                "같은 이름의 머신이 이미 있습니다. 다른 이름을 입력하세요."
            }
            (Self::En, Failure::Write) => {
                "The change could not be saved. Check that your account can write to the configuration folder, then try again."
            }
            (Self::Ko, Failure::Write) => {
                "변경 사항을 저장할 수 없습니다. 현재 계정의 설정 폴더 쓰기 권한을 확인하고 다시 시도하세요."
            }
            (Self::En, Failure::Worker) => {
                "The file operation stopped before it finished. Try again."
            }
            (Self::Ko, Failure::Worker) => {
                "파일 작업이 완료되기 전에 중단되었습니다. 다시 시도하세요."
            }
            (Self::En, Failure::Unavailable) => {
                "Machine management is available in the desktop app."
            }
            (Self::Ko, Failure::Unavailable) => "머신 관리는 데스크톱 앱에서 사용할 수 있습니다.",
            (Self::En, Failure::Unsent) => "Could not send. Try again.",
            (Self::Ko, Failure::Unsent) => "보내지 못했습니다. 다시 시도하세요.",
        };
        text.to_owned()
    }
}
