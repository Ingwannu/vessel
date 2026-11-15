//! Exact local persistence boundaries, not a new wire contract.
use super::*;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    route: BookRouteId,
}

#[test]
fn nonzero_ids_allocate_without_wrapping_or_reusing() {
    assert_eq!(BookRouteId::new(0), None);
    assert_eq!(BookRouteId::new(1), Some(BookRouteId::FIRST));
    assert_eq!(BookRouteId::new(u32::MAX), Some(BookRouteId::LAST));
    assert_eq!(BookRouteId::FIRST.next().unwrap().get(), 2);
    let before_last = BookRouteId::new(u32::MAX.checked_sub(1).unwrap()).unwrap();
    assert_eq!(before_last.next(), Some(BookRouteId::LAST));
    assert_eq!(BookRouteId::LAST.next(), None, "exhaustion is explicit");
}

#[test]
fn toml_keeps_both_bounds_as_exact_signed_integers() {
    for (id, number) in [(BookRouteId::FIRST, "1"), (BookRouteId::LAST, "4294967295")] {
        let saved = Saved { route: id };
        let text = toml_edit::ser::to_string(&saved).unwrap();
        assert_eq!(text, format!("route = {number}\n"));
        assert_eq!(toml_edit::de::from_str::<Saved>(&text).unwrap(), saved);
    }
    for number in ["0", "-1", "4294967296", "1.5", "\"1\""] {
        assert!(toml_edit::de::from_str::<Saved>(&format!("route = {number}\n")).is_err());
    }
}

#[test]
fn json_and_javascript_numbers_keep_every_id_exact_without_casts() {
    for id in [BookRouteId::FIRST, BookRouteId::LAST] {
        let text = serde_json::to_string(&id).unwrap();
        assert_eq!(text, id.get().to_string());
        assert_eq!(serde_json::from_str::<BookRouteId>(&text).unwrap(), id);
        let javascript_number = serde_json::from_str::<f64>(&text).unwrap();
        assert_eq!(javascript_number.to_bits(), f64::from(id.get()).to_bits());
    }
    // The maximum is below JavaScript's last exactly representable integer.
    assert!(f64::from(BookRouteId::LAST.get()) < 9_007_199_254_740_991.0);
    for invalid in ["0", "-1", "4294967296", "1.5", "\"1\"", "null"] {
        assert!(serde_json::from_str::<BookRouteId>(invalid).is_err());
    }
}
