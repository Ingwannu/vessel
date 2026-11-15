//! Draw the timeline-owned snapshot by reference; no preview catalog or transport.
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens,
    assets::Glyph,
    attachment::{Body, attachment_frame, attachment_tray},
    ui::{
        Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
        ParentElement as _, SharedString,
    },
};
use vessel_app_view::composer::{drafts::DraftKey, preview::Snapshot, tray::Origin};
use vessel_proto::AttachmentKind;

impl Shell {
    /// Capture exactly once before admission/preparation; the pending row owns it.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "W10 keyed ledger ingress consumes this snapshot in its coordinated cutover"
        )
    )]
    pub(crate) fn composer_preview(&self, target: DraftKey) -> Snapshot {
        Snapshot::capture(
            self.areas.composer.trays.get(target),
            self.areas.composer.contexts.get(target),
        )
    }

    /// The same kit file/image frames, without cloning the snapshot each frame.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "W10 pending-message renderer consumes this borrowed snapshot at cutover"
        )
    )]
    pub(crate) fn pending_attachment_preview(
        &self,
        snapshot: &Snapshot,
        tokens: &Tokens,
        cx: &mut Context<'_, Self>,
    ) -> Option<Div> {
        if snapshot.is_empty() {
            return None;
        }
        let files = snapshot.files().iter().enumerate().map(|(index, file)| {
            let source = (file.kind == AttachmentKind::Image)
                .then(|| super::pictures::picture(self, &file.source, &file.mime));
            let body = source.as_ref().map_or_else(
                || Body::Label {
                    icon: Glyph::FileText,
                    title: file.name.as_str().to_owned().into(),
                    subtitle: file.mime.as_str().to_owned().into(),
                },
                |source| Body::Image(Some(source.clone())),
            );
            let origin = file.source.clone();
            attachment_frame(&body, tokens)
                .id(SharedString::from(format!("pending-file-{index}")))
                .when_some(source, |frame, source| {
                    frame
                        .cursor_pointer()
                        .on_click(cx.listener(move |shell, _, window, cx| {
                            if let Origin::Stored(file) = &origin {
                                shell.open_stored_image(std::sync::Arc::clone(file), window, cx);
                            } else {
                                shell.open_local_image(source.clone(), window, cx);
                            }
                        }))
                })
                .into_any_element()
        });
        let contexts = snapshot.contexts().iter().enumerate().map(|(index, item)| {
            let body = Body::Label {
                icon: Glyph::FileText,
                title: item.title().to_owned().into(),
                subtitle: SharedString::default(),
            };
            let item = item.clone();
            attachment_frame(&body, tokens)
                .id(SharedString::from(format!("pending-context-{index}")))
                .cursor_pointer()
                .on_click(cx.listener(move |shell, _, window, cx| {
                    shell.open_context_snapshot(item.clone(), window, cx)
                }))
                .into_any_element()
        });
        Some(attachment_tray().children(files).children(contexts))
    }
}
