use prytty_core::{detect_language, AnsiWriter, ColorMode, Language};
use prytty_syntax::tokenize;
use wasm_bindgen::prelude::*;

/// Highlight text, return ANSI-colored string.
/// language: optional hint ("rust", "json", "diff", etc.)
/// If None/undefined, auto-detect from content.
#[wasm_bindgen]
pub fn highlight(text: &str, language: Option<String>) -> String {
    let lang = language
        .as_deref()
        .and_then(Language::from_name)
        .unwrap_or_else(|| detect_language(None, text));

    let tokens = tokenize(lang, text);
    // Always truecolor — Crytter's VTE handles all SGR sequences
    let writer = AnsiWriter::new(ColorMode::TrueColor, Default::default());
    writer.render(&tokens)
}

/// Detect the language of the given text. Returns the language name
/// or "generic" if no match.
#[wasm_bindgen]
pub fn detect(text: &str) -> String {
    detect_language(None, text).name().to_string()
}

/// List all supported language names.
#[wasm_bindgen]
pub fn languages() -> Vec<String> {
    vec![
        "rust".into(),
        "python".into(),
        "json".into(),
        "yaml".into(),
        "toml".into(),
        "diff".into(),
        "log".into(),
        "generic".into(),
    ]
}
