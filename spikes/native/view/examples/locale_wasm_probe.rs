//! Link the portable localization layer for artifact-size measurements,
//! without a UI toolkit or a node connection. Black boxes keep the plain
//! and variable access paths present in both formatting configurations.
use vessel_app_view::localization::{InitError, LANGUAGES, Number, PSEUDO, Translator};

fn main() -> Result<(), InitError> {
    for locale in LANGUAGES.iter().copied().chain([PSEUDO]) {
        let translator = Translator::get(locale)?;
        let _label = std::hint::black_box(translator.meta().language());
        let _welcome = std::hint::black_box(translator.examples().welcome("a-long-display-name")?);
        let _sessions = std::hint::black_box(translator.examples().sessions(Number::count(2))?);
    }
    Ok(())
}
