//! Fixed standalone View artifact, not a full browser runtime or app preview.
//! Run the ordinary generated boundary, rather than retaining an unrelated
//! kernel with black_box. Native reachability comes from NUMBER registration.

use vessel_app_view::localization::{ENGLISH, LANGUAGES, Number, Translator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for locale in LANGUAGES {
        let translator = Translator::get(*locale)?;
        let count = std::env::args()
            .nth(1)
            .and_then(|value| value.parse().ok())
            .unwrap_or(4);
        let sentence = translator.examples().grouped(Number::count(count))?;
        println!("{sentence}");
    }
    // The explicit English fallback cache is constructed on both variants.
    let _english = Translator::get(ENGLISH)?;
    Ok(())
}
