// MODIFIED by Vessel: share optional image activation across existing render paths.
//! The host may open an unlinked image without changing its layout or selection atom.

use std::sync::Arc;

use gpui::{
    App, ClickEvent, Img, SharedUri, Stateful, StatefulInteractiveElement as _, Styled as _, Window,
};

use super::node::LinkMark;
use super::text_view::{LinkClickHandlerFn, handle_link_click};

pub(super) type ImageClickHandlerFn =
    dyn Fn(&SharedUri, &ClickEvent, &mut Window, &mut App) + Send + Sync;

#[derive(Clone)]
pub(super) struct ImageActivation {
    pub(super) uri: SharedUri,
    pub(super) handler: Arc<ImageClickHandlerFn>,
}

/// Attach behavior to the same image element in both wrapping and scrolling layouts.
/// A host image handler owns activation; otherwise existing linked-image behavior stays.
pub(super) fn activate(
    image: Stateful<Img>,
    activation: Option<ImageActivation>,
    link: Option<LinkMark>,
    link_handler: Option<Arc<LinkClickHandlerFn>>,
) -> Stateful<Img> {
    if let Some(activation) = activation {
        let auxiliary = activation.clone();
        image
            .cursor_pointer()
            .on_click(move |event, window, cx| {
                crate::TextSelection::end(window, cx);
                cx.stop_propagation();
                (activation.handler)(&activation.uri, event, window, cx);
            })
            .on_aux_click(move |event, window, cx| {
                crate::TextSelection::end(window, cx);
                cx.stop_propagation();
                (auxiliary.handler)(&auxiliary.uri, event, window, cx);
            })
    } else if let Some(link) = link {
        let auxiliary_link = link.clone();
        let auxiliary_handler = link_handler.clone();
        image
            .cursor_pointer()
            .on_click(move |event, window, cx| {
                crate::TextSelection::end(window, cx);
                cx.stop_propagation();
                handle_link_click(&link_handler, link.target(), event.clone(), window, cx);
            })
            .on_aux_click(move |event, window, cx| {
                crate::TextSelection::end(window, cx);
                cx.stop_propagation();
                handle_link_click(
                    &auxiliary_handler,
                    auxiliary_link.target(),
                    event.clone(),
                    window,
                    cx,
                );
            })
    } else {
        image
    }
}
