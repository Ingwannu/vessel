//! A panic, as the owner's log file records it (one test: `init` sets the
//! process's subscriber and panic hook).

#[test]
fn a_panic_is_in_the_log_file_with_where_it_happened() {
    let dir = tempfile::tempdir().unwrap();
    let logging = vessel_app_log::init_in(Some(dir.path()), "0.0.0.0.6");
    assert_eq!(
        logging.file(),
        Some(dir.path().join(vessel_app_log::FILE).as_path())
    );
    // A span with a field, as zbus logs inside one: stderr colours it,
    // the file must not (0.0.0.0.6's file had stderr's escape codes).
    tracing::info_span!("caching", task_name = "proxy").in_scope(|| {
        tracing::warn!("inside a span");
    });
    let caught = std::panic::catch_unwind(|| panic!("boom 7f3c"));
    assert!(caught.is_err());
    let text = std::fs::read_to_string(dir.path().join(vessel_app_log::FILE)).unwrap();
    assert!(
        text.contains("Vessel starts") && text.contains("0.0.0.0.6"),
        "{text}"
    );
    assert!(
        text.contains("the app panicked") && text.contains("boom 7f3c"),
        "{text}"
    );
    assert!(text.contains("panic.rs"), "no place in:\n{text}");
    assert!(
        text.contains("task_name=\"proxy\""),
        "no span field in:\n{text}"
    );
    assert!(
        !text.contains('\u{1b}'),
        "colour codes in the file:\n{text}"
    );
    // Windows: the frames as addresses in the exe, for the kept .pdb.
    if cfg!(windows) {
        assert!(
            text.contains("frames=\"exe+0x"),
            "no frame addresses in:\n{text}"
        );
    }
}
