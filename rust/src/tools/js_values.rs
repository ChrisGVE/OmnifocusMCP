//! Rust values rendered as JavaScript literals for the tool script builders.
//!
//! Every builder in `crate::tools` splices its arguments into an OmniJS
//! script as literals. Strings go through `crate::jxa::escape_for_jxa`, so
//! user text can never be read as code, and an absent optional argument
//! becomes `null`, which the scripts test for.

use crate::jxa::escape_for_jxa;

/// An optional string as a JavaScript string literal, or `null`.
pub(crate) fn js_string_or_null(value: Option<&str>) -> String {
    value
        .map(escape_for_jxa)
        .unwrap_or_else(|| "null".to_string())
}

/// An optional string, trimmed, as a JavaScript string literal, or `null`.
pub(crate) fn js_trimmed_string_or_null(value: Option<&str>) -> String {
    value
        .map(|text| escape_for_jxa(text.trim()))
        .unwrap_or_else(|| "null".to_string())
}

/// An optional boolean as `true`, `false` or `null`.
pub(crate) fn js_bool_or_null(value: Option<bool>) -> String {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    }
    .to_string()
}

/// An optional number in Rust's `Display` form, or `null`.
pub(crate) fn js_number_or_null<T: ToString>(value: Option<T>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "null".to_string())
}
