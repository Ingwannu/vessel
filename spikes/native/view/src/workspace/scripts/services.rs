//! Paseo's health tones and URL preference are pure presentation policy.
//! This is not a health probe or proxy implementation: only a future reviewed
//! node report can supply health/routes. Current nodes remain explicitly absent.
use super::RunState;
use vessel_proto::HttpUrl;
use vessel_proto::scripts::ScriptKind;

/// A node report's three health states, separate from process lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    /// Probe grace or no conclusive report yet.
    Pending,
    /// Node's own service probe succeeded.
    Healthy,
    /// Node's own failure threshold was reached.
    Unhealthy,
}
/// Status colours follow the reference without inferring health from `Running`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// No live process.
    Idle,
    /// A live process with no conclusive health claim.
    Running,
    /// Every running service in this summary reports health.
    Healthy,
    /// At least one running service reports failure.
    Unhealthy,
}
/// Which independently reachable route the person prefers for this host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteKind {
    /// Node's public reverse proxy endpoint.
    Public,
    /// Memorable local proxy hostname, supplied by the node.
    Proxy,
    /// Explicit/direct service endpoint.
    Direct,
}
/// One validated HTTP target; the kind is presentation, never routing authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteTarget {
    /// Meaning of the supplied endpoint.
    pub kind: RouteKind,
    /// Credentials cannot be embedded in this type.
    pub url: HttpUrl,
}
/// Route availability is distinct from an old node that reports no such facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceReport {
    /// Peer/runtime has no service-proxy/health contract yet.
    Unavailable,
    /// Facts reported by a compatible node, with independently absent routes.
    Reported {
        /// TCP health from the node, never a client-side inference.
        health: Health,
        /// Validated targets sorted by reference priority by `routes`.
        targets: Vec<RouteTarget>,
    },
}
/// Health affects running services only; stopped entries always remain idle.
#[must_use]
pub fn tone(kind: ScriptKind, state: RunState, health: Option<Health>) -> Tone {
    let live = match state {
        RunState::Running => true,
        RunState::Starting
        | RunState::Backoff
        | RunState::Idle
        | RunState::Exited(_)
        | RunState::Failed
        | RunState::Stopped
        | RunState::Unknown => false,
    };
    if !live {
        return Tone::Idle;
    }
    match kind {
        ScriptKind::Script => Tone::Running,
        ScriptKind::Service => match health {
            Some(Health::Healthy) => Tone::Healthy,
            Some(Health::Unhealthy) => Tone::Unhealthy,
            Some(Health::Pending) | None => Tone::Running,
        },
    }
}
/// Unhealthy dominates; all running services must report healthy for green.
/// Plain running scripts do not dilute the service-only health summary.
#[must_use]
pub fn summary(rows: impl IntoIterator<Item = (ScriptKind, Tone)>) -> Tone {
    let mut any_running = false;
    let mut service_seen = false;
    let mut all_healthy = true;
    let mut unhealthy = false;
    for (kind, tone) in rows {
        if tone == Tone::Idle {
            continue;
        }
        any_running = true;
        match kind {
            ScriptKind::Service => {
                service_seen = true;
                all_healthy &= tone == Tone::Healthy;
                unhealthy |= tone == Tone::Unhealthy;
            }
            ScriptKind::Script => {}
        }
    }
    if !any_running {
        Tone::Idle
    } else if unhealthy {
        Tone::Unhealthy
    } else if service_seen && all_healthy {
        Tone::Healthy
    } else {
        Tone::Running
    }
}
/// Priority public, proxy, direct; duplicate URLs keep their first meaning.
#[must_use]
pub fn routes(
    public: Option<HttpUrl>,
    proxy: Option<HttpUrl>,
    direct: Option<HttpUrl>,
) -> Vec<RouteTarget> {
    let mut targets: Vec<RouteTarget> = Vec::new();
    for (kind, url) in [
        (RouteKind::Public, public),
        (RouteKind::Proxy, proxy),
        (RouteKind::Direct, direct),
    ] {
        if let Some(url) = url
            && !targets.iter().any(|target| target.url == url)
        {
            targets.push(RouteTarget { kind, url });
        }
    }
    targets
}
/// A missing preferred route falls back to the primary available target.
#[must_use]
pub fn selected(targets: &[RouteTarget], preferred: Option<RouteKind>) -> Option<&RouteTarget> {
    targets
        .iter()
        .find(|target| Some(target.kind) == preferred)
        .or_else(|| targets.first())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn health_never_colours_stopped_entries_or_plain_scripts() {
        for kind in [ScriptKind::Script, ScriptKind::Service] {
            for health in [
                None,
                Some(Health::Pending),
                Some(Health::Healthy),
                Some(Health::Unhealthy),
            ] {
                assert_eq!(tone(kind, RunState::Stopped, health), Tone::Idle);
                if kind == ScriptKind::Script {
                    assert_eq!(tone(kind, RunState::Running, health), Tone::Running);
                }
            }
        }
        assert_eq!(
            tone(ScriptKind::Service, RunState::Running, None),
            Tone::Running
        );
        assert_eq!(
            tone(
                ScriptKind::Service,
                RunState::Running,
                Some(Health::Healthy)
            ),
            Tone::Healthy
        );
        assert_eq!(
            tone(
                ScriptKind::Service,
                RunState::Running,
                Some(Health::Unhealthy)
            ),
            Tone::Unhealthy
        );
    }
    #[test]
    fn a_report_from_a_previous_child_cannot_colour_starting_or_backoff_as_healthy() {
        for state in [
            RunState::Starting,
            RunState::Backoff,
            RunState::Unknown,
            RunState::Exited(Some(0)),
            RunState::Exited(Some(7)),
            RunState::Failed,
        ] {
            assert_eq!(
                tone(ScriptKind::Service, state, Some(Health::Healthy)),
                Tone::Idle
            );
            assert_eq!(
                tone(ScriptKind::Service, state, Some(Health::Unhealthy)),
                Tone::Idle
            );
        }
    }
    #[test]
    fn summary_requires_all_services_to_report_health_and_failure_dominates() {
        assert_eq!(summary([]), Tone::Idle);
        assert_eq!(
            summary([
                (ScriptKind::Script, Tone::Running),
                (ScriptKind::Service, Tone::Healthy)
            ]),
            Tone::Healthy
        );
        assert_eq!(
            summary([
                (ScriptKind::Service, Tone::Running),
                (ScriptKind::Service, Tone::Healthy)
            ]),
            Tone::Running
        );
        assert_eq!(
            summary([
                (ScriptKind::Service, Tone::Unhealthy),
                (ScriptKind::Service, Tone::Healthy)
            ]),
            Tone::Unhealthy
        );
    }
    #[test]
    fn route_order_deduplication_and_unavailable_preference_follow_paseo() {
        let public: HttpUrl = "https://service.example.invalid".parse().unwrap();
        let direct: HttpUrl = "http://localhost:3000".parse().unwrap();
        let targets = routes(Some(public.clone()), Some(public), Some(direct));
        assert_eq!(
            targets.iter().map(|target| target.kind).collect::<Vec<_>>(),
            vec![RouteKind::Public, RouteKind::Direct]
        );
        assert_eq!(
            selected(&targets, Some(RouteKind::Proxy)).unwrap().kind,
            RouteKind::Public
        );
        assert_eq!(
            selected(&targets, Some(RouteKind::Direct)).unwrap().kind,
            RouteKind::Direct
        );
        assert!(selected(&[], None).is_none());
        assert!(
            "https://user:password@example.invalid"
                .parse::<HttpUrl>()
                .is_err()
        );
    }
}
