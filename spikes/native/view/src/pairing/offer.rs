//! Safe invitation endpoint facts, prepared once outside Render. These are
//! claims from an invitation, not authenticated connection evidence.

use std::sync::Arc;

use crate::Lang;
use crate::connections::{Endpoint, Route};

/// An offer's routes in preference order, with shared prepared words.
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

/// One typed safe endpoint and its prepared bilingual row title.
#[derive(Debug)]
pub struct OfferRoute {
    route: Route,
    en: Arc<str>,
    ko: Arc<str>,
}

impl OfferRoute {
    fn new(route: Route) -> Self {
        Self {
            en: Arc::from(Lang::En.overview().route(&route)),
            ko: Arc::from(Lang::Ko.overview().route(&route)),
            route,
        }
    }

    /// Typed endpoint facts, never the original URL.
    #[must_use]
    pub const fn route(&self) -> &Route {
        &self.route
    }

    /// Shared text ready for a row, without formatting during Render.
    #[must_use]
    pub fn words(&self, lang: Lang) -> &Arc<str> {
        match lang {
            Lang::En => &self.en,
            Lang::Ko => &self.ko,
        }
    }
}

#[cfg(test)]
mod tests;
