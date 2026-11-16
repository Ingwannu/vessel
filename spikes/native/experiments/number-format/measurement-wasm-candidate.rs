//! Fixed candidate entry through the same generated API used by native callers.
//! This is a standalone artifact probe, not a browser runtime or a live preview.

use vessel_app_view::localization::{AmountUnit, LANGUAGES, Number, Translator};
use vessel_proto::conversation::FiniteNumber;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let count = std::env::args()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(u64::MAX);
    let value = std::env::args()
        .nth(2)
        .and_then(|value| value.parse().ok())
        .unwrap_or(f64::MAX);
    let value = FiniteNumber::new(value)?;
    for locale in LANGUAGES {
        let translator = Translator::get(*locale)?;
        println!(
            "{}",
            translator.examples().grouped(Number::unsigned(count))?
        );
        for unit in [
            AmountUnit::Usd,
            AmountUnit::CompactTokens,
            AmountUnit::Quantity,
        ] {
            println!(
                "{}",
                translator.examples().grouped(Number::amount(value, unit))?
            );
        }
    }
    Ok(())
}
