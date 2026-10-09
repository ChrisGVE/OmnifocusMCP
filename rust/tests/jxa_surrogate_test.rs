//! Unpaired UTF-16 surrogates in OmniFocus data (audit CR-042).
//!
//! JavaScript strings are UTF-16 and may hold half of a surrogate pair on its
//! own, for example a note cut in the middle of an emoji. OmniJS's
//! `JSON.stringify` writes such a half as an escape like `\ud83d`, and
//! serde_json refuses that text, so one damaged string failed a whole read.
//! Before parsing, each unpaired surrogate escape is replaced with `\ufffd`,
//! the Unicode replacement character; a valid high + low pair is kept.

use std::borrow::Cow;

use omnifocus_mcp::jxa::{parse_jxa_output, replace_lone_surrogate_escapes};
use serde_json::{json, Value};

/// Sanitizes `json_text` and parses it.
fn parsed(json_text: &str) -> Value {
    let cleaned = replace_lone_surrogate_escapes(json_text);
    serde_json::from_str(&cleaned).expect("sanitized text parses")
}

// ------------------------------------------------ unpaired halves are replaced

#[test]
fn a_lone_high_surrogate_becomes_the_replacement_character() {
    assert_eq!(parsed(r#""a\ud83db""#), json!("a\u{FFFD}b"));
}

#[test]
fn a_lone_low_surrogate_becomes_the_replacement_character() {
    assert_eq!(parsed(r#""a\ude00b""#), json!("a\u{FFFD}b"));
}

#[test]
fn upper_case_hex_digits_are_recognised() {
    assert_eq!(parsed(r#""\uD800""#), json!("\u{FFFD}"));
    assert_eq!(parsed(r#""\uDFFF""#), json!("\u{FFFD}"));
}

#[test]
fn a_high_half_at_the_end_of_a_string_is_replaced() {
    assert_eq!(
        parsed(r#"{"note":"cut\ud83d"}"#),
        json!({"note": "cut\u{FFFD}"})
    );
}

#[test]
fn a_high_half_followed_by_a_non_surrogate_escape_is_replaced_alone() {
    assert_eq!(parsed(r#""\ud800\u0041""#), json!("\u{FFFD}A"));
}

#[test]
fn a_low_half_before_a_high_half_is_two_lone_halves() {
    assert_eq!(parsed(r#""\udc00\ud800""#), json!("\u{FFFD}\u{FFFD}"));
}

#[test]
fn a_lone_high_half_before_a_valid_pair_leaves_the_pair_intact() {
    assert_eq!(parsed(r#""\ud800\ud83d\ude00""#), json!("\u{FFFD}😀"));
}

#[test]
fn the_replacement_is_the_escape_of_the_replacement_character() {
    assert_eq!(
        replace_lone_surrogate_escapes(r#""x\udbffy""#),
        r#""x\ufffdy""#
    );
}

// ------------------------------------------------ everything else is kept

#[test]
fn a_valid_pair_is_kept() {
    let text = r#""smile \ud83d\ude00""#;
    assert_eq!(parsed(text), json!("smile 😀"));
    assert!(matches!(
        replace_lone_surrogate_escapes(text),
        Cow::Borrowed(_)
    ));
}

#[test]
fn a_valid_pair_in_upper_case_is_kept() {
    assert_eq!(parsed(r#""\uD83D\uDE00""#), json!("😀"));
}

#[test]
fn other_escapes_are_kept() {
    let text = r#""caf\u00e9 \u20ac \n \" \/""#;
    assert_eq!(replace_lone_surrogate_escapes(text), text);
}

#[test]
fn an_escaped_backslash_before_u_is_not_an_escape() {
    // The JSON text `\\ud800` is a backslash followed by the letters `ud800`.
    let text = r#""\\ud800""#;
    assert_eq!(replace_lone_surrogate_escapes(text), text);
    assert_eq!(parsed(text), json!("\\ud800"));
}

#[test]
fn an_escaped_backslash_then_a_lone_half_replaces_only_the_half() {
    assert_eq!(parsed(r#""\\\ud800""#), json!("\\\u{FFFD}"));
}

#[test]
fn text_without_escapes_is_borrowed_unchanged() {
    let text = r#"{"ok":true,"data":["é", 1]}"#;
    assert!(matches!(replace_lone_surrogate_escapes(text), Cow::Borrowed(t) if t == text));
}

#[test]
fn an_incomplete_escape_at_the_end_is_left_for_the_parser() {
    let text = r#""\ud8"#;
    assert_eq!(replace_lone_surrogate_escapes(text), text);
}

// ------------------------------------------------ the parse path uses it

#[test]
fn osascript_output_with_a_lone_surrogate_parses() {
    let value = parse_jxa_output(r#"{"ok":true,"data":[{"name":"Plan \ud83d"},{"name":"ok"}]}"#)
        .expect("one damaged string must not fail the whole read");
    assert_eq!(
        value,
        json!({"ok": true, "data": [{"name": "Plan \u{FFFD}"}, {"name": "ok"}]})
    );
}

#[test]
fn osascript_output_with_a_valid_pair_parses_to_the_character() {
    let value = parse_jxa_output(r#"{"ok":true,"data":"😀"}"#).expect("valid JSON");
    assert_eq!(value, json!({"ok": true, "data": "😀"}));
}
