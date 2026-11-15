//! Cache form display inputs at updates; widget painting never validates text,
//! translates a diagnostic or computes future times itself.
use super::{
    fields::Field,
    io,
    preview::Problem,
    session::{Model, Visit},
};
use crate::schedules::{
    clock::{Clock, ClockError},
    presentation::Notice,
};
use crate::{Lang, localization::Translator};
use std::sync::Arc;

/// A cached editable text field and its own post-edit validation feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    /// The field source selector; callback identity remains the actual editor.
    pub field: Field,
    /// Already localized label.
    pub label: Arc<str>,
    /// Existing cached input, with no further normalization.
    pub value: Arc<str>,
    /// Source placeholder, never submitted as an input value.
    pub placeholder: Arc<str>,
    /// Only this edited field's failure, never a pristine required warning.
    pub error: Option<Arc<str>>,
}
/// Comparable form words and canonical node preview; no widget handles here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    /// Original form/host visit captured by UI controls.
    pub visit: Visit,
    /// New schedule, edited schedule or heartbeat heading.
    pub title: Arc<str>,
    /// Submit words for the source footer's right button.
    pub submit: Arc<str>,
    /// Empty for cadence-only heartbeat edits; source name/prompt/max order.
    pub text: Vec<Text>,
    /// Current rule description uses the accepted legacy cadence table once.
    pub cadence: Option<Arc<str>>,
    /// Cached grammar failure, not a raw parser diagnostic.
    pub cadence_error: Option<Arc<str>>,
    /// Only matching node-projected times, shown in the explicit viewer clock.
    pub next_runs: Vec<Arc<str>>,
    /// Unknown viewer identity keeps the explicit UTC display fallback visible.
    pub viewer_known: bool,
    /// The exact operation's typed error; App chooses the source surface.
    pub problems: Vec<OperationProblem>,
    /// Known insufficient authority; unknown grants never imply read-only.
    pub read_only: Option<Arc<str>>,
    /// Current parsed definition and matching-preview readiness.
    pub can_save: bool,
    /// Input engines also become read-only while a save/retirement blocks them.
    pub blocked: bool,
    /// Loading content on the submit button is independent of close policy.
    pub saving: bool,
}
/// Error belongs to one real slot/method, not a guessed global transport state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationProblem {
    /// Project/agent lookup.
    Targets(Notice),
    /// Selected agent's dynamic options.
    Options(Notice),
    /// Rule preview, including its typed node reason.
    Preview(Notice),
    /// Authoritative save refusal or uncertain completion.
    Save(Notice),
}
/// Prepare only an actual open visit; caller caches the result on data edits.
/// # Errors
/// Preserve calendar/catalog failures rather than silently replacing a zone.
pub fn prepare(
    model: &Model,
    clock: &Clock,
    catalog: &Translator,
) -> Result<Option<Form>, ClockError> {
    let (Some(visit), Some(draft)) = (model.visit(), model.draft()) else {
        return Ok(None);
    };
    let words = catalog.schedules();
    let lang = Lang::from_tag(catalog.locale().tag());
    let title = if draft.opened().is_none() {
        words.form_new_title()
    } else if draft.fields().is_none() {
        words.form_heartbeat_title()
    } else {
        words.form_edit_title()
    };
    let text = draft.fields().map_or_else(Vec::new, |fields| {
        [
            (
                Field::Name,
                words.form_name(),
                fields.name(),
                words.form_optional(),
            ),
            (
                Field::Prompt,
                words.form_prompt(),
                fields.prompt(),
                words.form_prompt_placeholder(),
            ),
            (
                Field::Limit,
                words.form_limit(),
                fields.limit(),
                words.form_unlimited(),
            ),
        ]
        .into_iter()
        .map(|(field, label, value, placeholder)| Text {
            field,
            label: label.into(),
            value: value.into(),
            placeholder: placeholder.into(),
            error: fields
                .feedback(field)
                .map(|error| error.label(catalog).into()),
        })
        .collect()
    });
    let (cadence, cadence_error) = match draft.cadence.checked() {
        Ok(value) => (Some(lang.schedule_cadence(value).into()), None),
        Err(error) => (None, Some(lang.cadence_error(error).into())),
    };
    let next_runs = model.projected().map_or_else(
        || Ok(Vec::new()),
        |value| {
            value
                .runs
                .iter()
                .map(|at| -> Result<Arc<str>, ClockError> {
                    Ok(words.form_next_at(&clock.at(*at)?)?.into())
                })
                .collect::<Result<Vec<_>, _>>()
        },
    )?;
    let problems = model
        .operations()
        .filter_map(|slot| {
            let request = slot.request()?;
            let failure = slot.failure()?;
            let label = match request.operation() {
                io::Operation::Preview(_) => Problem::of(failure.clone()).label(catalog),
                io::Operation::Targets | io::Operation::Options(_) | io::Operation::Save(_) => {
                    crate::schedules::locale::failure(failure, catalog)
                }
            };
            let notice = Notice {
                label: label.into(),
                variant: failure.variant(),
            };
            Some(match request.operation() {
                io::Operation::Targets => OperationProblem::Targets(notice),
                io::Operation::Options(_) => OperationProblem::Options(notice),
                io::Operation::Preview(_) => OperationProblem::Preview(notice),
                io::Operation::Save(_) => OperationProblem::Save(notice),
            })
        })
        .collect();
    Ok(Some(Form {
        visit,
        title: title.into(),
        submit: if draft.opened().is_some() {
            words.form_save()
        } else {
            words.form_create()
        }
        .into(),
        text,
        cadence,
        cadence_error,
        next_runs,
        viewer_known: clock.is_known(),
        problems,
        read_only: model
            .authority()
            .and_then(|authority| authority.explanation(lang))
            .map(Into::into),
        can_save: model.prepare_save().ok().flatten().is_some(),
        blocked: model.blocked(),
        saving: model.pending_save(),
    }))
}

#[cfg(test)]
mod tests;
