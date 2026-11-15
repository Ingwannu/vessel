//! Device layout follows the existing acknowledged book-edit seam.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_view::layout::{HostAlias, LayoutError};
use vessel_app_view::machine_book::Edit;
impl Shell {
    pub(crate) fn layout_book_applied(&mut self, edit: &Edit, cx: &mut Context<'_, Self>) {
        super::initialized(self);
        let result = (|| {
            let mut book = self.areas.layout.book.borrow_mut();
            let book = book.as_mut().ok_or(LayoutError::InvalidState)?;
            match edit {
                Edit::Rename { from, to } => vessel_app_view::layout::rename_alias(
                    book,
                    &HostAlias::parse(from.as_str())?,
                    &HostAlias::parse(to.as_str())?,
                ),
                Edit::Forget(alias) => {
                    vessel_app_view::layout::forget_alias(book, &HostAlias::parse(alias.as_str())?);
                    Ok(())
                }
                Edit::MakeDefault(_) => Ok(()),
            }
        })();
        if !matches!(edit, Edit::MakeDefault(_)) {
            self.layout_result(result, cx);
        }
    }
}
