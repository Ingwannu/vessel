use super::*;

#[test]
fn level_refuses_foreign_nonfinite_and_out_of_range_measurements() {
    for value in [f32::NAN, f32::INFINITY, -0.1, 1.01] {
        assert!(Level::new(value).is_err());
    }
    assert_eq!(Level::new(0.0).unwrap(), Level::default());
    assert!(Level::new(1.0).is_ok());
}

#[test]
fn publication_uses_last_published_state_and_the_exact_interval_boundary() {
    let mut meter = DisplayMeter::default();
    let quiet = Level::new(0.02).unwrap();
    for time in [1, 30, 60, 119] {
        assert_eq!(meter.observe(quiet, Duration::from_millis(time)), None);
    }
    let published = meter.observe(quiet, Duration::from_millis(120)).unwrap();
    assert!(published.get().mul_add(-1.0, 0.007).abs() < 0.000_001);
    assert!(
        meter
            .observe(Level::new(1.0).unwrap(), Duration::from_millis(121))
            .is_some()
    );
    let before = meter
        .telemetry(Mute::Open, SpeechActivity::Quiet)
        .level
        .get();
    let released = meter
        .observe(Level::default(), Duration::from_millis(122))
        .unwrap()
        .get();
    assert!(released < before);
    let muted = meter.telemetry(Mute::Muted, SpeechActivity::Speaking);
    assert_eq!(muted.level.get().to_bits(), released.to_bits());
    assert_eq!(muted.activity, SpeechActivity::Speaking);
    assert_eq!(muted.mute, Mute::Muted);
}
