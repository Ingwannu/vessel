//! Retry admission belongs to one already failed read, never a visible row index.
use super::{CatalogStatus, Failure, Received, active, at, io, ready};
#[test]
fn retry_uses_original_context_fresh_ticket_and_rejects_duplicate_or_retired_actions() {
    let (mut model, opened) = active();
    assert!(model.retry_read(opened.visit, &opened.request).is_none());
    assert!(matches!(
        model.received(&opened.request, Err(Failure::TimedOut)),
        Received::Changed
    ));
    let retry = model.retry_read(opened.visit, &opened.request).unwrap();
    assert_eq!(retry.connection(), opened.request.connection());
    assert_eq!(retry.operation(), opened.request.operation());
    assert_ne!(retry.ticket(), opened.request.ticket());
    assert!(model.retry_read(opened.visit, &opened.request).is_none());
    assert!(matches!(
        model.received(&retry, Err(Failure::Busy)),
        Received::Changed
    ));
    assert!(model.close(opened.visit));
    assert!(model.retry_read(opened.visit, &retry).is_none());
}
#[test]
fn a_failed_write_or_unsupported_preview_is_not_a_generic_read_retry() {
    let (mut model, opened) = active();
    let preview = model.preview(opened.visit, at()).unwrap().unwrap();
    assert!(matches!(
        model.received(&preview, Err(Failure::Older)),
        Received::Changed
    ));
    assert!(model.retry_read(opened.visit, &preview).is_none());
    let candidate = ready(&mut model, opened.visit);
    let save = model.save(opened.visit, candidate).unwrap();
    assert!(matches!(
        model.received(&save, Err(Failure::TimedOut)),
        Received::Changed
    ));
    assert!(model.retry_read(opened.visit, &save).is_none());
}

#[test]
fn model_retry_retains_the_failure_until_its_exact_new_read_settles() {
    let (mut model, opened) = active();
    assert!(matches!(
        model.received(
            &opened.request,
            Ok(io::Value::Targets(Box::new(super::choices::targets())))
        ),
        Received::Changed
    ));
    let request = model
        .browse_agent(opened.visit, "claude-code".parse().unwrap())
        .unwrap();
    assert!(matches!(model.catalog_status(), CatalogStatus::Loading));
    let original = model.draft().unwrap().target().unwrap().spec().clone();
    assert!(matches!(
        model.received(&request, Err(Failure::Busy)),
        Received::Changed
    ));
    assert!(matches!(
        model.catalog_status(),
        CatalogStatus::Failed(Failure::Busy)
    ));
    let retry = model.retry_read(opened.visit, &request).unwrap();
    assert_ne!(retry.ticket(), request.ticket());
    assert_eq!(retry.connection(), request.connection());
    assert_eq!(retry.operation(), request.operation());
    assert!(matches!(
        model.catalog_status(),
        CatalogStatus::Retrying(Failure::Busy)
    ));
    assert!(model.retry_read(opened.visit, &request).is_none());
    assert!(model.retry_read(opened.visit, &retry).is_none());
    assert!(matches!(
        model.received(&request, Ok(io::Value::Options(super::choices::options()))),
        Received::Ignored
    ));
    assert!(matches!(
        model.catalog_status(),
        CatalogStatus::Retrying(Failure::Busy)
    ));
    assert_eq!(model.draft().unwrap().target().unwrap().spec(), &original);
    assert!(matches!(
        model.received(&retry, Ok(io::Value::Options(super::choices::options()))),
        Received::Changed
    ));
    assert!(matches!(model.catalog_status(), CatalogStatus::Ready));
    assert_eq!(model.draft().unwrap().target().unwrap().spec(), &original);
}
