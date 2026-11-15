//! Source choice search policy and localized feedback, using the prepared cache.
use super::{Picker, Shell};
const SEARCH_THRESHOLD: usize = 6;
pub(super) fn searchable(shell: &Shell, picker: Picker) -> bool {
    match picker {
        Picker::Project | Picker::Model | Picker::Zone => true,
        Picker::Host | Picker::Cadence | Picker::Unit => false,
        Picker::Thinking | Picker::Mode => shell
            .areas
            .schedules
            .data
            .editor_cache
            .as_ref()
            .ok()
            .and_then(Option::as_ref)
            .and_then(|form| form.controls.as_ref())
            .and_then(|controls| match picker {
                Picker::Thinking => controls.effort.as_ref(),
                Picker::Mode => controls.permissions.as_ref(),
                Picker::Host
                | Picker::Project
                | Picker::Model
                | Picker::Cadence
                | Picker::Unit
                | Picker::Zone => None,
            })
            .is_some_and(|control| control.values.len() > SEARCH_THRESHOLD),
    }
}
pub(super) fn search_placeholder(shell: &Shell, picker: Picker) -> &str {
    let words = shell.words.schedules();
    match picker {
        Picker::Project => words.form_search_projects(),
        Picker::Model => words.form_search_models(),
        Picker::Thinking => words.form_search_thinking(),
        Picker::Mode => words.form_search_modes(),
        Picker::Zone => words.form_search_zones(),
        Picker::Host | Picker::Cadence | Picker::Unit => "",
    }
}
pub(super) fn empty_label(shell: &Shell, picker: Picker) -> &str {
    let words = shell.words.schedules();
    match picker {
        Picker::Host => words.search_empty(),
        Picker::Project => words.form_empty_projects(),
        Picker::Model => words.form_empty_models(),
        Picker::Thinking => words.form_empty_thinking(),
        Picker::Mode => words.form_empty_modes(),
        Picker::Zone => words.form_empty_zones(),
        Picker::Cadence | Picker::Unit => words.form_empty_choices(),
    }
}
