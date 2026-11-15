//! Retry admission belongs to one already failed read, never a visible row index.
use super::{Failure, Received, active, at, ready};
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
