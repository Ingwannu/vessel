//! Staged reducer proofs: cached reads start no executable, and fake observations
//! cannot overwrite a newer definition or forced refresh.

use super::cache::*;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use vessel_harness::providers::{
    ConfigurationRequirement, PluginDefinition, PluginRevision, ProviderRegistry,
};
use vessel_proto::{
    agents::AgentVersion,
    providers::{
        Catalogue, CatalogueDirectory, CatalogueScope, DiscoveryFailure, Enabled, LaunchSettings,
        ModeCatalogue, Models, ProviderId, ProviderOverride, ProviderOverrides, ProviderState,
        UnavailableReason,
    },
};

fn registry() -> ProviderRegistry {
    ProviderRegistry::build(&ProviderOverrides::default(), &[]).unwrap()
}
fn id(id: &str) -> ProviderId {
    id.parse().unwrap()
}
fn time() -> DateTime<Utc> {
    "2026-01-01T00:00:00Z".parse().unwrap()
}
fn catalogue() -> Catalogue {
    Catalogue {
        models: Models::default(),
        modes: ModeCatalogue::new(vec![], None).unwrap(),
    }
}
fn ready() -> Observation {
    Observation::Ready {
        catalogue: catalogue(),
        version: Some("1.2.3".parse::<AgentVersion>().unwrap()),
    }
}
fn start(cache: &mut Cache, provider: &str, scope: &CatalogueScope, refresh: Refresh) -> Work {
    let Admission::Start(work) = cache.begin(&id(provider), scope, refresh).unwrap() else {
        panic!("expected admitted fake work")
    };
    work
}
fn workspace(name: &str) -> CatalogueScope {
    CatalogueScope::Workspace(CatalogueDirectory::new(std::env::temp_dir().join(name)).unwrap())
}

#[test]
fn cached_diagnostics_do_not_launch_or_consume_target_capacity() {
    let mut cache = Cache::new(registry());
    for n in 0..=MAX_CATALOGUES {
        let read = cache
            .cached(&id("claude"), &workspace(&format!("fake-{n}")))
            .unwrap();
        assert_eq!(
            read.entry.state,
            ProviderState::Unavailable {
                reason: UnavailableReason::NotChecked
            }
        );
        assert_eq!(read.version, None);
    }
    assert_eq!(
        cache.snapshot(&CatalogueScope::Global).unwrap().len(),
        registry().definitions().len()
    );
    assert!(matches!(
        cache
            .begin(&id("claude"), &CatalogueScope::Global, Refresh::Cached)
            .unwrap(),
        Admission::Start(_)
    ));
    assert!(matches!(
        cache.cached(&id("unknown"), &CatalogueScope::Global),
        Err(CacheError::UnknownProvider)
    ));
}

#[test]
fn full_execution_budget_still_serves_disabled_ready_and_joined_reads() {
    let mut cache = Cache::new(registry());
    let scope = CatalogueScope::Global;
    let work = start(&mut cache, "claude", &scope, Refresh::Cached);
    assert!(
        matches!(cache.admit(&id("claude"), &scope, Refresh::Cached, WorkBudget::Full), Ok(Admission::Joined(ticket)) if ticket == work.ticket)
    );
    assert!(matches!(
        cache.admit(&id("claude"), &scope, Refresh::Force, WorkBudget::Full),
        Err(CacheError::Busy)
    ));
    assert!(cache.complete(&work, ready(), time()).is_some());
    assert!(matches!(
        cache.admit(&id("claude"), &scope, Refresh::Cached, WorkBudget::Full),
        Ok(Admission::Ready(_))
    ));
    assert!(matches!(
        cache.admit(&id("omp"), &scope, Refresh::Force, WorkBudget::Full),
        Ok(Admission::Ready(_))
    ));
    assert!(matches!(
        cache.admit(&id("codex"), &scope, Refresh::Cached, WorkBudget::Full),
        Err(CacheError::Busy)
    ));
    assert_eq!(
        cache.cached(&id("codex"), &scope).unwrap().entry.state,
        ProviderState::Unavailable {
            reason: UnavailableReason::NotChecked
        }
    );
}

#[test]
fn host_catalogues_coalesce_and_target_catalogues_stay_distinct() {
    let mut cache = Cache::new(registry());
    let first = start(&mut cache, "claude", &workspace("first"), Refresh::Cached);
    let Admission::Joined(joined) = cache
        .begin(&id("claude"), &workspace("second"), Refresh::Cached)
        .unwrap()
    else {
        panic!("one host-scoped observation")
    };
    assert_eq!(joined, first.ticket);
    let observed = cache.complete(&first, ready(), time()).unwrap();
    let Admission::Ready(cached) = cache
        .begin(&id("claude"), &workspace("third"), Refresh::Cached)
        .unwrap()
    else {
        panic!("cached host result")
    };
    assert_eq!(cached, observed);
    assert_eq!(cached.version, Some("1.2.3".parse().unwrap()));
    let a = start(&mut cache, "opencode", &workspace("first"), Refresh::Cached);
    let b = start(
        &mut cache,
        "opencode",
        &workspace("second"),
        Refresh::Cached,
    );
    assert_ne!(a.ticket, b.ticket);
    assert_ne!(a.scope, b.scope);
}

#[test]
fn a_late_result_from_before_a_forced_refresh_cannot_publish() {
    let mut cache = Cache::new(registry());
    let old = start(
        &mut cache,
        "codex",
        &CatalogueScope::Global,
        Refresh::Cached,
    );
    let new = start(&mut cache, "codex", &CatalogueScope::Global, Refresh::Force);
    assert!(cache.complete(&old, ready(), time()).is_none());
    let current = cache
        .complete(
            &new,
            Observation::Failed(DiscoveryFailure::ProbeFailed),
            time(),
        )
        .unwrap();
    assert_eq!(
        current.entry.state,
        ProviderState::Error {
            failure: DiscoveryFailure::ProbeFailed
        }
    );
    assert!(cache.complete(&old, ready(), time()).is_none());
    assert_eq!(
        cache.cached(&id("codex"), &CatalogueScope::Global).unwrap(),
        current
    );
}

#[test]
fn disabled_providers_are_not_admitted_and_unavailability_is_not_an_empty_catalogue() {
    let mut cache = Cache::new(registry());
    let Admission::Ready(disabled) = cache
        .begin(&id("omp"), &CatalogueScope::Global, Refresh::Force)
        .unwrap()
    else {
        panic!("disabled must not start")
    };
    assert_eq!(
        disabled.entry.state,
        ProviderState::Unavailable {
            reason: UnavailableReason::Disabled
        }
    );
    let work = start(&mut cache, "pi", &CatalogueScope::Global, Refresh::Cached);
    let result = cache
        .complete(
            &work,
            Observation::Unavailable(UnavailableReason::DriverMissing),
            time(),
        )
        .unwrap();
    assert_eq!(
        result.entry.state,
        ProviderState::Unavailable {
            reason: UnavailableReason::DriverMissing
        }
    );
}

#[test]
fn generation_replacement_retains_unchanged_work_and_refuses_changed_results() {
    let mut cache = Cache::new(registry());
    let unchanged = start(
        &mut cache,
        "codex",
        &CatalogueScope::Global,
        Refresh::Cached,
    );
    let changed = start(
        &mut cache,
        "claude",
        &CatalogueScope::Global,
        Refresh::Cached,
    );
    let overrides = ProviderOverrides::new(BTreeMap::from([(
        id("claude"),
        ProviderOverride {
            enabled: Some(Enabled::Off),
            ..ProviderOverride::default()
        },
    )]))
    .unwrap();
    let cancelled = cache.replace(ProviderRegistry::build(&overrides, &[]).unwrap());
    assert_eq!(cancelled, [changed.ticket]);
    assert!(cache.complete(&changed, ready(), time()).is_none());
    assert!(cache.complete(&unchanged, ready(), time()).is_some());
    assert_eq!(
        cache
            .cached(&id("claude"), &CatalogueScope::Global)
            .unwrap()
            .entry
            .state,
        ProviderState::Unavailable {
            reason: UnavailableReason::Disabled
        }
    );
}

#[test]
fn plugin_replacement_invalidates_only_its_registration_generation() {
    let mut plugin = PluginDefinition {
        id: id("custom-plugin"),
        label: "Plugin".parse().unwrap(),
        configuration: ConfigurationRequirement::Optional,
        revision: PluginRevision::new(1),
        launch: LaunchSettings::default(),
    };
    let mut cache = Cache::new(
        ProviderRegistry::build(&ProviderOverrides::default(), &[plugin.clone()]).unwrap(),
    );
    let work = start(
        &mut cache,
        "custom-plugin",
        &CatalogueScope::Global,
        Refresh::Cached,
    );
    plugin.revision = PluginRevision::new(2);
    assert_eq!(
        cache.replace(ProviderRegistry::build(&ProviderOverrides::default(), &[plugin]).unwrap()),
        [work.ticket]
    );
    assert!(cache.complete(&work, ready(), time()).is_none());
}

#[test]
fn scope_growth_is_bounded_without_evicting_an_inflight_observation() {
    let mut cache = Cache::new(registry());
    for n in 0..MAX_CATALOGUES {
        let _work = start(
            &mut cache,
            "opencode",
            &workspace(&format!("bound-{n}")),
            Refresh::Cached,
        );
    }
    assert!(matches!(
        cache.begin(&id("opencode"), &workspace("one-more"), Refresh::Cached),
        Err(CacheError::Capacity)
    ));
}

/// A compact Arc-shared observation can expand drastically on the wire.
fn shared_thinking_catalogue(model_count: usize) -> Catalogue {
    use vessel_proto::{
        ModelId, OptionValueId,
        providers::{MAX_CHOICE_ID, Model, ModelSpec, ThinkingChoice, ThinkingChoices},
    };
    let choices = ThinkingChoices::new(
        (0..128)
            .map(|index| ThinkingChoice {
                id: OptionValueId::new(format!("{index:04}{}", "x".repeat(MAX_CHOICE_ID - 4))),
                label: "Choice".parse().unwrap(),
                default: None,
            })
            .collect(),
    )
    .unwrap();
    Catalogue {
        models: Models::new(
            (0..model_count)
                .map(|index| {
                    Model::new(ModelSpec {
                        id: ModelId::new(format!("model-{index}")),
                        label: "Model".parse().unwrap(),
                        default: None,
                        visibility: None,
                        thinking: Some(choices.clone()),
                        default_thinking: None,
                    })
                    .unwrap()
                })
                .collect(),
        )
        .unwrap(),
        modes: ModeCatalogue::new(vec![], None).unwrap(),
    }
}

#[test]
fn shared_thinking_wire_expansion_cannot_publish_an_oversized_ready_catalogue() {
    let mut cache = Cache::new(registry());
    let scope = CatalogueScope::Global;
    let work = start(&mut cache, "codex", &scope, Refresh::Cached);
    let completed = cache
        .complete(
            &work,
            Observation::Ready {
                catalogue: shared_thinking_catalogue(128),
                version: Some("1.2.3".parse().unwrap()),
            },
            time(),
        )
        .unwrap();
    assert!(matches!(
        completed.entry.state,
        ProviderState::Error {
            failure: DiscoveryFailure::InvalidCatalogue
        }
    ));
    assert_eq!(cache.cached(&id("codex"), &scope).unwrap(), completed);
    assert!(super::budget::fits(&completed));
}

#[test]
fn an_oversized_aggregate_refuses_without_losing_individually_readable_diagnostics() {
    let mut cache = Cache::new(registry());
    let scope = CatalogueScope::Global;
    for provider in ["codex", "claude"] {
        let work = start(&mut cache, provider, &scope, Refresh::Cached);
        let observed = cache
            .complete(
                &work,
                Observation::Ready {
                    catalogue: shared_thinking_catalogue(43),
                    version: None,
                },
                time(),
            )
            .unwrap();
        assert!(matches!(observed.entry.state, ProviderState::Ready { .. }));
        assert!(super::budget::fits(&observed));
    }
    assert!(matches!(
        cache.snapshot(&scope),
        Err(CacheError::ReplyTooLarge)
    ));
    for provider in ["codex", "claude"] {
        let diagnostic = cache.cached(&id(provider), &scope).unwrap();
        assert!(matches!(
            diagnostic.entry.state,
            ProviderState::Ready { .. }
        ));
        assert!(super::budget::fits(&diagnostic));
    }
}
