//! Metadata route selection (ADR 0062) from one immutable parsed snapshot.
//! It reads no file/key or socket; all-candidate settings validation belongs to preparation.

use super::config::{Provider, Route, Routes};
use std::fmt;
use vessel_proto::settings::{MetadataEffort, MetadataProvider, MetadataSettings};
use vessel_proto::{ModelId, ModelRouteName};

/// A tuple checked against one immutable catalog, borrowing its owners.
pub(super) struct Selection<'a> {
    route: &'a Route,
    provider: &'a Provider,
    model: &'a ModelId,
    effort: Option<MetadataEffort>,
}
impl Selection<'_> {
    /// The checked route for call accounting.
    #[must_use]
    pub(super) const fn route(&self) -> &Route {
        self.route
    }
    /// The corresponding provider; credentials are read only by the caller.
    #[must_use]
    pub(super) const fn provider(&self) -> &Provider {
        self.provider
    }
    /// The actual checked target, not an unrelated small model.
    #[must_use]
    pub(super) const fn model(&self) -> &ModelId {
        self.model
    }
    /// The preference for the later API representability/budget check.
    #[must_use]
    pub(super) const fn effort(&self) -> Option<MetadataEffort> {
        self.effort
    }
}
// A provider includes an endpoint and key name. Selecting it must not make
// those values diagnostic output through a derived Debug higher up.
impl fmt::Debug for Selection<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Selection")
            .field("route", &self.route.name)
            .field("model", &self.model)
            .field("effort", &self.effort)
            .finish_non_exhaustive()
    }
}

/// No call is made when a selected preference cannot be honored.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum SelectError {
    /// No current route resolves, without falling out of manual mode.
    #[error("no model route is available for generated names")]
    NoRoute,
    /// A manual setting must resolve before it is kept.
    #[error("metadata route {0} is not available")]
    MissingRoute(ModelRouteName),
    /// A newer saved effort is readable but cannot be acted on.
    #[error("metadata requests an unknown effort")]
    UnknownEffort,
    /// Reload changed a selected route's allowed pair.
    #[error("metadata's model is not allowed by route {0}")]
    ModelNotAllowed(ModelRouteName),
}

/// Choose one tuple; the caller still validates the encoded API request.
/// No retry, credential access or automatic append is hidden in selection.
///
/// # Errors
/// No route resolves, or the selected manual tuple has a disallowed model
/// or an unknown effort. A later API-specific budget check is still required.
pub(super) fn select<'a>(
    catalog: &'a Routes,
    settings: &'a MetadataSettings,
    current: Option<&ModelRouteName>,
) -> Result<Selection<'a>, SelectError> {
    if !settings.is_automatic() {
        for candidate in settings.providers() {
            if let Some((route, provider)) = catalog.get(&candidate.route) {
                return manual(route, provider, candidate);
            }
        }
        return Err(SelectError::NoRoute);
    }
    for hint in Hint::ORDER {
        for route in catalog.entries() {
            if let Some(model) = hint.model(route)
                && let Some((route, provider)) = catalog.get(&route.name)
            {
                return checked(route, provider, model, hint.effort());
            }
        }
    }
    let (route, provider) = current
        .and_then(|name| catalog.get(name))
        .ok_or(SelectError::NoRoute)?;
    checked(route, provider, default_model(route), None)
}

/// Validate one stored manual entry without automatic fallback or I/O.
pub(super) fn candidate<'a>(
    catalog: &'a Routes,
    entry: &'a MetadataProvider,
) -> Result<Selection<'a>, SelectError> {
    let (route, provider) = catalog
        .get(&entry.route)
        .ok_or_else(|| SelectError::MissingRoute(entry.route.clone()))?;
    manual(route, provider, entry)
}

fn manual<'a>(
    route: &'a Route,
    provider: &'a Provider,
    candidate: &'a MetadataProvider,
) -> Result<Selection<'a>, SelectError> {
    checked(
        route,
        provider,
        candidate
            .model
            .as_ref()
            .unwrap_or_else(|| default_model(route)),
        candidate.effort,
    )
}

fn checked<'a>(
    route: &'a Route,
    provider: &'a Provider,
    model: &'a ModelId,
    effort: Option<MetadataEffort>,
) -> Result<Selection<'a>, SelectError> {
    if !route.allows(model.as_str()) {
        return Err(SelectError::ModelNotAllowed(route.name.clone()));
    }
    if effort == Some(MetadataEffort::Unknown) {
        return Err(SelectError::UnknownEffort);
    }
    Ok(Selection {
        route,
        provider,
        model,
        effort,
    })
}

fn default_model(route: &Route) -> &ModelId {
    route.small_model.as_ref().unwrap_or(&route.model)
}

#[derive(Clone, Copy)]
enum Hint {
    Haiku,
    GptMini,
    MiniMax,
    Nemotron,
}
impl Hint {
    const ORDER: &[Self] = &[Self::Haiku, Self::GptMini, Self::MiniMax, Self::Nemotron];
    const fn needle(self) -> &'static str {
        match self {
            Self::Haiku => "haiku",
            Self::GptMini => "gpt-5.4-mini",
            Self::MiniMax => "minimax-m3",
            Self::Nemotron => "nemotron-3-super",
        }
    }
    const fn effort(self) -> Option<MetadataEffort> {
        match self {
            Self::GptMini => Some(MetadataEffort::Low),
            Self::Haiku | Self::MiniMax | Self::Nemotron => None,
        }
    }
    fn model(self, route: &Route) -> Option<&ModelId> {
        let matches = |text: &str| text.to_lowercase().contains(self.needle());
        if matches(route.model.as_str()) {
            Some(&route.model)
        } else if let Some(small) = route.small_model.as_ref().filter(|id| matches(id.as_str())) {
            Some(small)
        } else if matches(route.name.as_str()) {
            Some(default_model(route))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
