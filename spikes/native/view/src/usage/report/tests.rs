use super::*;
use crate::localization::ENGLISH;
use vessel_proto::{
    TokenCount, TurnUsage,
    conversation::{Cost, FiniteNumber},
};

fn tokens() -> TurnUsage {
    TurnUsage {
        total: TokenCount::new(150),
        input: TokenCount::new(100),
        output: TokenCount::new(50),
        thought: None,
        cached_read: Some(TokenCount::new(0)),
        cached_write: None,
    }
}
fn cost(amount: f64, unit: &str) -> Cost {
    Cost {
        amount: FiniteNumber::new(amount).unwrap(),
        unit: unit.parse().unwrap(),
    }
}
#[test]
fn ledger_field_coverage_and_cost_units_remain_independent() {
    let words = Translator::get(ENGLISH).unwrap();
    let totals = UsageTotals {
        tokens: Some(tokens()),
        reported: ReportCount::new(2),
        unreported: ReportCount::new(1),
        optional_reports: OptionalUsageReports {
            cached_read: ReportCount::new(1),
            ..Default::default()
        },
        costs: vec![cost(1.25, "USD"), cost(2.5, "credits")],
        unaccounted_costs: ReportCount::new(1),
        ..Default::default()
    };
    let facts = total_facts(&totals, CostVisibility::Permitted, words);
    let total = facts.iter().find(|f| f.label == "Total tokens").unwrap();
    let cached = facts
        .iter()
        .find(|f| f.label == "Cache read tokens")
        .unwrap();
    let thought = facts
        .iter()
        .find(|f| f.label == "Reasoning tokens")
        .unwrap();
    assert_eq!(total.value, "150");
    assert!(total.detail.as_ref().unwrap().contains('2'));
    assert_eq!(cached.value, "0");
    assert!(cached.detail.as_ref().unwrap().contains('1'));
    assert_eq!(thought.value, "Not reported");
    assert!(
        facts
            .iter()
            .any(|f| f.label.contains("USD") && f.value == "1.25")
    );
    assert!(
        facts
            .iter()
            .any(|f| f.label.contains("credits") && f.value == "2.5")
    );
    assert!(
        facts
            .iter()
            .any(|f| f.label == "Unattributed cost reports" && f.value == "1")
    );
}
#[test]
fn withheld_costs_hide_even_an_accidentally_populated_wire_value() {
    let words = Translator::get(ENGLISH).unwrap();
    let totals = UsageTotals {
        costs: vec![cost(123.25, "USD")],
        ..Default::default()
    };
    let facts = total_facts(&totals, CostVisibility::Withheld, words);
    assert!(facts.iter().any(|f| f.value == "Hidden for this device"));
    assert!(
        facts
            .iter()
            .all(|f| !f.value.contains("123.25") && !f.label.contains("USD"))
    );
}
