//! Resource ownership and formatting match the core without importing its keys.
use super::super::{Date, ENGLISH, Locale, Number, PSEUDO, Translator, resolution::resolve};
use super::{Bundles, bundles};
use fluent_bundle::{FluentArgs, FluentResource};
use std::sync::Arc;

fn source(locale: Locale) -> &'static str {
    if locale == Locale::from_tag("ko").unwrap() {
        include_str!("../../../../locales/ko/examples.ftl")
    } else {
        include_str!("../../../../locales/en/examples.ftl")
    }
}

fn value(pair: &Bundles<Arc<FluentResource>>, key: &'static str, args: &FluentArgs<'_>) -> String {
    resolve(&pair.localized, &pair.fallback, key, Some(args)).unwrap()
}

#[test]
fn arc_owned_bundles_share_all_builtins_and_the_pseudo_transform() {
    for locale in [ENGLISH, Locale::from_tag("ko").unwrap(), PSEUDO] {
        let mut pair = bundles::<Arc<FluentResource>>(locale).unwrap();
        assert!(pair.localized.get_message("meta-name").is_none());
        assert!(pair.fallback.get_message("meta-name").is_none());
        let resource = Arc::new(FluentResource::try_new(source(locale).to_owned()).unwrap());
        let weak = Arc::downgrade(&resource);
        pair.localized.add_resource(resource).unwrap();
        pair.fallback
            .add_resource(Arc::new(
                FluentResource::try_new(source(ENGLISH).to_owned()).unwrap(),
            ))
            .unwrap();
        // A contribution's lookalike key is never a DATETIME pattern override.
        pair.localized
            .add_resource(Arc::new(
                FluentResource::try_new("formats-date = Wrong date".to_owned()).unwrap(),
            ))
            .unwrap();
        let core = Translator::get(locale).unwrap().examples();
        let mut args = FluentArgs::new();
        args.set("count", Number::count(1_234_567).fluent());
        assert_eq!(
            value(&pair, "examples-grouped", &args),
            core.grouped(Number::count(1_234_567)).unwrap()
        );
        let date = Date::Day(chrono::NaiveDate::from_ymd_opt(2026, 11, 13).unwrap());
        args.set("date", date.fluent().unwrap());
        assert_eq!(
            value(&pair, "examples-date", &args),
            core.date(&date).unwrap()
        );
        args.set("name", "서울");
        assert_eq!(
            value(&pair, "examples-remove", &args),
            core.remove("서울").unwrap()
        );
        drop(pair);
        assert!(weak.upgrade().is_none());
    }
}

#[test]
fn a_missing_plugin_message_uses_english_plural_rules_in_its_own_bundle() {
    let mut pair = bundles::<Arc<FluentResource>>(Locale::from_tag("ko").unwrap()).unwrap();
    pair.fallback
        .add_resource(Arc::new(
            FluentResource::try_new(source(ENGLISH).to_owned()).unwrap(),
        ))
        .unwrap();
    for count in [1, 2] {
        let mut args = FluentArgs::new();
        args.set("count", Number::count(count).fluent());
        assert_eq!(
            value(&pair, "examples-sessions", &args),
            Translator::get(ENGLISH)
                .unwrap()
                .examples()
                .sessions(Number::count(count))
                .unwrap()
        );
    }
}
