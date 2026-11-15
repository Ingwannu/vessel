//! Machine selector decisions, separate from its toolkit's drawing/search field.
use super::{MachineFilter, Page};
use crate::localization::Translator;

/// One searchable host choice, already named and checked.
#[derive(Debug, PartialEq, Eq)]
pub struct MachineChoice {
    /// Value sent back on selection.
    pub filter: MachineFilter,
    /// Whole translated label, or validated book key.
    pub label: String,
    /// Current selection.
    pub checked: bool,
}
impl Page {
    /// Label of the selected machine filter.
    #[must_use]
    pub fn machine_label<'a>(&'a self, catalog: &'a Translator) -> &'a str {
        match &self.machine {
            MachineFilter::All => catalog.schedules().all_machines(),
            MachineFilter::One(key) => key.as_str(),
        }
    }
    /// Search is case-insensitive for both the all-hosts text and book names.
    #[must_use]
    pub fn machine_choices(&self, catalog: &Translator, query: &str) -> Vec<MachineChoice> {
        let query = query.trim().to_lowercase();
        std::iter::once(MachineFilter::All)
            .chain(self.keys().cloned().map(MachineFilter::One))
            .filter_map(|filter| {
                let label = match &filter {
                    MachineFilter::All => catalog.schedules().all_machines().to_owned(),
                    MachineFilter::One(key) => key.as_str().to_owned(),
                };
                label
                    .to_lowercase()
                    .contains(&query)
                    .then(|| MachineChoice {
                        checked: filter == self.machine,
                        filter,
                        label,
                    })
            })
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::super::tests::key;
    use super::*;
    fn en() -> &'static Translator {
        Translator::get(crate::localization::ENGLISH).unwrap()
    }
    fn ko() -> &'static Translator {
        Translator::get(crate::localization::Locale::from_tag("ko").unwrap()).unwrap()
    }
    #[test]
    fn machine_search_and_selection_do_not_depend_on_a_renderer() {
        let mut page = Page::default();
        page.machines(&[key("Laptop"), key("build-server")]);
        assert_eq!(
            page.machine_choices(en(), " LAP "),
            [MachineChoice {
                filter: MachineFilter::One(key("Laptop")),
                label: "Laptop".into(),
                checked: false
            }]
        );
        assert_eq!(
            page.machine_choices(ko(), "모든"),
            [MachineChoice {
                filter: MachineFilter::All,
                label: "모든 머신".into(),
                checked: true
            }]
        );
        assert_eq!(page.machine_label(en()), "All machines");
        page.machine = MachineFilter::One(key("Laptop"));
        assert_eq!(page.machine_label(ko()), "Laptop");
        assert!(page.machine_choices(en(), "not-here").is_empty());
    }
}

impl super::Page {
    /// Grant evidence for this exact admission. New forms reuse the host's
    /// self-report; edits also keep any actual refusal recorded for that row.
    #[must_use]
    pub fn authority_for(
        &self,
        connection: &crate::connection::ConnectionContext,
        schedule: Option<vessel_proto::ScheduleId>,
    ) -> Option<super::super::authority::Authority> {
        let cache = self.caches.get(connection.machine().as_str())?;
        if cache.connection.as_ref() != Some(connection) {
            return None;
        }
        if let Some(row) = schedule.and_then(|id| cache.rows.get(&id)) {
            return Some(row.authority.clone());
        }
        Some(
            cache
                .scopes
                .as_ref()
                .map_or(super::super::authority::Authority::Unknown, |scopes| {
                    super::super::authority::Authority::Scopes(scopes.clone())
                }),
        )
    }
}
