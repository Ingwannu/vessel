//! A composer's attached files (track P's row c): each upload's steps
//! land in its composer's tray (`vessel_app_view::composer::tray`), which
//! the pills draw and Send waits on. The files are sent by the link
//! (`link::attachments`), a piece at a time, beside its other commands.

use vessel_app_kit::ui::Context;
use vessel_app_link::attachments::Step;
use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::composer::tray::ItemKey;
use vessel_proto::SessionId;

use crate::shell::Shell;

impl Shell {
    /// A step of a file's upload to `session`: its tray follows (a tray
    /// dropped since, or a file removed since, takes nothing).
    pub(crate) fn composer_upload(
        &mut self,
        session: SessionId,
        key: ItemKey,
        step: Step,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(tray) = self
            .areas
            .composer
            .trays
            .get_mut(DraftKey::Session { session })
        else {
            return;
        };
        match step {
            Step::Progress(received) => tray.progress(key, received),
            Step::Done(attachment) => tray.uploaded(key, attachment),
            Step::Failed { problem, again } => {
                tracing::info!(?problem, ?again, "a file's upload failed");
                tray.failed(key, problem, again);
            }
        }
        cx.notify();
    }
}
