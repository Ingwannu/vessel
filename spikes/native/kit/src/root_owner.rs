//! The kit owns its root wrapper's content boundary. Host callbacks may compare
//! identities here without depending on the component layer or retaining views.
use gpui::{AnyView, App, EntityId};

/// Check original content through the kit's root wrapper before routing an OS
/// callback. A retained old entity must not receive a replacement root's action.
#[must_use]
pub fn root_contains_view(root: &AnyView, view: EntityId, cx: &App) -> bool {
    root.entity_id() == view
        || root
            .clone()
            .downcast::<gpui_component::Root>()
            .is_ok_and(|root| root.read(cx).view().entity_id() == view)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, Context, IntoElement, Render, TestAppContext, Window, div};

    struct Content;
    impl Render for Content {
        fn render(
            &mut self,
            _window: &mut Window,
            _cx: &mut Context<'_, Self>,
        ) -> impl IntoElement {
            div()
        }
    }

    #[gpui::test]
    fn direct_and_wrapped_content_match_only_the_original_entity(cx: &mut TestAppContext) {
        cx.update(|cx| {
            crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap();
        });
        let (content, cx) = cx.add_window_view(|_, _| Content);
        content.update_in(cx, |_, window, cx| {
            let original = cx.entity();
            let other = cx.new(|_| Content);
            let direct: AnyView = original.clone().into();
            let wrapped: AnyView = crate::root(original.clone(), window, cx).into();
            assert!(root_contains_view(&direct, original.entity_id(), cx));
            assert!(root_contains_view(&wrapped, original.entity_id(), cx));
            assert!(!root_contains_view(&direct, other.entity_id(), cx));
            assert!(!root_contains_view(&wrapped, other.entity_id(), cx));
        });
    }
}
