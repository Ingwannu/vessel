//! Safe invitation endpoint facts, prepared once outside Render. These are
//! claims from an invitation, not authenticated connection evidence.

use std::sync::Arc;

use crate::Lang;
use crate::connections::{Endpoint, Route};

/// An offer's routes in preference order, with their prepared row words.
#[derive(Debug, Clone)]
pub struct OfferRoutes(Arc<[OfferRoute]>);

impl OfferRoutes {
    /// Project only endpoint facts; URL credentials and resource paths never
    /// enter this model. The original wire offer retains all effect inputs.
    #[must_use]
    pub fn of(offer: &vessel_proto::PairingOffer) -> Self {
        let direct = offer
            .direct
            .iter()
            .map(|url| Route::Direct(Endpoint::of_url(url.as_str())));
        let relays = offer
            .relays
            .iter()
            .map(|url| Route::Relay(Endpoint::of_url(url.as_str())));
        Self(direct.chain(relays).map(OfferRoute::new).collect())
    }

    /// Direct routes first, then relays; an empty offer has no route rows.
    #[must_use]
    pub fn rows(&self) -> &[OfferRoute] {
        &self.0
    }
}

/// One typed safe endpoint, its kind's words in both languages and its
/// prepared endpoint text: the row's title and its value (Paseo's
/// `DetailRow`: "Relay" over the endpoint).
#[derive(Debug)]
pub struct OfferRoute {
    route: Route,
    kind_en: &'static str,
    kind_ko: &'static str,
    endpoint: Arc<str>,
}

impl OfferRoute {
    fn new(route: Route) -> Self {
        Self {
            kind_en: Lang::En.overview().route_kind(&route),
            kind_ko: Lang::Ko.overview().route_kind(&route),
            endpoint: Arc::from(route.endpoint().as_str()),
            route,
        }
    }

    /// Typed endpoint facts, never the original URL.
    #[must_use]
    pub const fn route(&self) -> &Route {
        &self.route
    }

    /// The route's kind ("TCP", "Relay" / "릴레이"): the row's title.
    #[must_use]
    pub const fn kind(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::En => self.kind_en,
            Lang::Ko => self.kind_ko,
        }
    }

    /// The endpoint ("host:port"), prepared once: the row's value.
    #[must_use]
    pub const fn endpoint(&self) -> &Arc<str> {
        &self.endpoint
    }
}

#[cfg(test)]
mod tests;
