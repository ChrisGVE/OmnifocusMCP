//! Behavioural tests for the OmniJS date helpers (`JS_DATE_HELPERS`), run in a
//! real JavaScriptCore engine.
//!
//! OmniFocus evaluates our scripts on JavaScriptCore, so the date semantics
//! under test (how `new Date(...)` reads a string, local vs UTC, calendar
//! rollover) are the engine's, not something a Rust mock could reproduce. The
//! macOS system shell `jsc` is a plain JS engine: it never touches OmniFocus.
//! Each test pins the time zone through the `TZ` environment variable, which
//! `jsc` honours, so the expected UTC instants are deterministic.
//!
//! The script is passed with `jsc -e`, so no temporary files are written.

use std::{path::Path, process::Command};

use omnifocus_mcp::js_helpers::JS_DATE_HELPERS;

const JSC_PATH: &str =
    "/System/Library/Frameworks/JavaScriptCore.framework/Versions/Current/Helpers/jsc";

const MELBOURNE: &str = "Australia/Melbourne";
const NEW_YORK: &str = "America/New_York";

/// Runs `JS_DATE_HELPERS` followed by `snippet` in `jsc` under time zone
/// `time_zone` and returns everything the snippet printed, trimmed.
///
/// Fails loudly when `jsc` is missing: these tests must not silently skip.
fn run_js(time_zone: &str, snippet: &str) -> String {
    assert!(
        Path::new(JSC_PATH).exists(),
        "JavaScriptCore shell not found at {JSC_PATH}; these tests require macOS"
    );
    let script = format!("{JS_DATE_HELPERS}\n{snippet}");
    let output = Command::new(JSC_PATH)
        .arg("-e")
        .arg(&script)
        .env("TZ", time_zone)
        .output()
        .expect("jsc should launch");
    let stdout = String::from_utf8(output.stdout).expect("jsc stdout should be UTF-8");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "jsc exited with {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status
    );
    stdout.trim().to_string()
}

/// Evaluates a JS expression yielding a Date and returns its UTC ISO string.
fn iso_of(time_zone: &str, expression: &str) -> String {
    run_js(time_zone, &format!("print(({expression}).toISOString());"))
}

/// Evaluates a JS expression expected to throw and returns the error message.
fn error_of(time_zone: &str, expression: &str) -> String {
    run_js(
        time_zone,
        &format!(
            r#"try {{ const value = {expression}; print("NO ERROR: " + String(value)); }}
catch (error) {{ print("ERROR: " + error.message); }}"#
        ),
    )
}

/// Declares a fake OmniJS `settings` global whose `objectForKey` returns
/// `value_literal` (a JS literal) for every key and records the keys asked.
fn fake_settings(value_literal: &str) -> String {
    format!(
        r#"var askedKeys = [];
var settings = {{ objectForKey: function (key) {{ askedKeys.push(key); return {value_literal}; }} }};"#
    )
}

// ---------------------------------------------------------------- parseLocalDate

#[test]
fn bare_date_is_local_midnight_in_melbourne() {
    // Upstream #13 repro: a bare date used to land at 10:00 local in UTC+10.
    assert_eq!(
        iso_of(MELBOURNE, r#"parseLocalDate("2026-07-12", "deferBefore")"#),
        "2026-07-11T14:00:00.000Z"
    );
}

#[test]
fn bare_date_is_local_midnight_in_new_york() {
    // Negative offset: used to land on the previous evening locally.
    assert_eq!(
        iso_of(NEW_YORK, r#"parseLocalDate("2026-07-12", "dueBefore")"#),
        "2026-07-12T04:00:00.000Z"
    );
}

#[test]
fn bare_date_surrounded_by_whitespace_is_trimmed() {
    assert_eq!(
        iso_of(MELBOURNE, r#"parseLocalDate("  2026-07-12 ", "dueBefore")"#),
        "2026-07-11T14:00:00.000Z"
    );
}

#[test]
fn bare_leap_day_is_accepted() {
    assert_eq!(
        iso_of(NEW_YORK, r#"parseLocalDate("2028-02-29", "dueBefore")"#),
        "2028-02-29T05:00:00.000Z"
    );
}

#[test]
fn bare_date_with_two_digit_year_is_not_shifted_to_1900s() {
    // new Date(50, 0, 1) means 1950; the helper must keep year 0050.
    assert_eq!(
        iso_of("UTC", r#"parseLocalDate("0050-01-01", "dueBefore")"#),
        "0050-01-01T00:00:00.000Z"
    );
}

#[test]
fn utc_date_time_is_kept_as_given() {
    assert_eq!(
        iso_of(
            MELBOURNE,
            r#"parseLocalDate("2026-07-12T09:30:00Z", "dueBefore")"#
        ),
        "2026-07-12T09:30:00.000Z"
    );
}

#[test]
fn offset_date_time_is_kept_as_given() {
    assert_eq!(
        iso_of(
            NEW_YORK,
            r#"parseLocalDate("2026-07-12T09:30:00+02:00", "dueBefore")"#
        ),
        "2026-07-12T07:30:00.000Z"
    );
}

#[test]
fn date_time_without_offset_is_local() {
    assert_eq!(
        iso_of(
            MELBOURNE,
            r#"parseLocalDate("2026-07-12T09:30:00", "dueBefore")"#
        ),
        "2026-07-11T23:30:00.000Z"
    );
}

#[test]
fn impossible_calendar_day_throws_with_field_name() {
    assert_eq!(
        error_of(MELBOURNE, r#"parseLocalDate("2026-02-30", "deferDate")"#),
        r#"ERROR: deferDate must be YYYY-MM-DD or an ISO 8601 date-time; received "2026-02-30""#
    );
}

#[test]
fn non_leap_year_february_29_throws() {
    assert_eq!(
        error_of(MELBOURNE, r#"parseLocalDate("2026-02-29", "dueDate")"#),
        r#"ERROR: dueDate must be YYYY-MM-DD or an ISO 8601 date-time; received "2026-02-29""#
    );
}

#[test]
fn empty_string_throws() {
    assert_eq!(
        error_of(MELBOURNE, r#"parseLocalDate("", "dueBefore")"#),
        r#"ERROR: dueBefore must be YYYY-MM-DD or an ISO 8601 date-time; received """#
    );
}

#[test]
fn garbage_throws() {
    assert_eq!(
        error_of(
            MELBOURNE,
            r#"parseLocalDate("next tuesday", "changed_after")"#
        ),
        r#"ERROR: changed_after must be YYYY-MM-DD or an ISO 8601 date-time; received "next tuesday""#
    );
}

#[test]
fn non_string_value_throws() {
    assert_eq!(
        error_of(MELBOURNE, r#"parseLocalDate(null, "absoluteDate")"#),
        "ERROR: absoluteDate must be YYYY-MM-DD or an ISO 8601 date-time; received null"
    );
}

// --------------------------------------------------------- parseOptionalLocalDate

#[test]
fn optional_local_date_passes_null_through() {
    assert_eq!(
        run_js(
            MELBOURNE,
            r#"print(parseOptionalLocalDate(null, "dueBefore") === null);"#
        ),
        "true"
    );
}

#[test]
fn optional_local_date_passes_undefined_through_as_null() {
    assert_eq!(
        run_js(
            MELBOURNE,
            r#"print(parseOptionalLocalDate(undefined, "dueBefore") === null);"#
        ),
        "true"
    );
}

#[test]
fn optional_local_date_parses_a_present_value_as_local() {
    assert_eq!(
        iso_of(
            MELBOURNE,
            r#"parseOptionalLocalDate("2026-07-12", "dueBefore")"#
        ),
        "2026-07-11T14:00:00.000Z"
    );
}

#[test]
fn optional_local_date_rejects_an_invalid_present_value() {
    assert_eq!(
        error_of(
            MELBOURNE,
            r#"parseOptionalLocalDate("bad-date", "dueBefore")"#
        ),
        r#"ERROR: dueBefore must be YYYY-MM-DD or an ISO 8601 date-time; received "bad-date""#
    );
}

// ---------------------------------------------------------------- parseWriteDate

#[test]
fn write_date_uses_configured_due_time() {
    let snippet = format!(
        r#"{}
print(parseWriteDate("2026-07-12", "dueDate", "DefaultDueTime", "17:00").toISOString());
print(askedKeys.join(","));"#,
        fake_settings(r#"key === "DefaultDueTime" ? "17:00" : null"#)
    );
    assert_eq!(
        run_js(MELBOURNE, &snippet),
        "2026-07-12T07:00:00.000Z\nDefaultDueTime"
    );
}

#[test]
fn write_date_uses_configured_non_default_time() {
    let snippet = format!(
        r#"{}
print(parseWriteDate("2026-07-12", "dueDate", "DefaultDueTime", "17:00").toISOString());"#,
        fake_settings(r#""08:30""#)
    );
    assert_eq!(run_js(MELBOURNE, &snippet), "2026-07-11T22:30:00.000Z");
}

#[test]
fn write_date_accepts_single_digit_hour_setting() {
    let snippet = format!(
        r#"{}
print(parseWriteDate("2026-07-12", "deferDate", "DefaultStartTime", "00:00").toISOString());"#,
        fake_settings(r#""8:05""#)
    );
    assert_eq!(run_js(MELBOURNE, &snippet), "2026-07-11T22:05:00.000Z");
}

#[test]
fn write_date_uses_factory_due_default_when_settings_is_undefined() {
    assert_eq!(
        iso_of(
            MELBOURNE,
            r#"parseWriteDate("2026-07-12", "dueDate", "DefaultDueTime", "17:00")"#
        ),
        "2026-07-12T07:00:00.000Z"
    );
}

#[test]
fn write_date_uses_factory_defer_default_when_settings_is_undefined() {
    // Upstream #13 repro on the write path: defer lands at local midnight.
    assert_eq!(
        iso_of(
            MELBOURNE,
            r#"parseWriteDate("2026-07-12", "deferDate", "DefaultStartTime", "00:00")"#
        ),
        "2026-07-11T14:00:00.000Z"
    );
}

#[test]
fn write_date_uses_factory_default_in_new_york() {
    assert_eq!(
        iso_of(
            NEW_YORK,
            r#"parseWriteDate("2026-07-12", "dueDate", "DefaultDueTime", "17:00")"#
        ),
        "2026-07-12T21:00:00.000Z"
    );
}

#[test]
fn write_date_uses_factory_default_when_settings_is_null() {
    let snippet = r#"var settings = null;
print(parseWriteDate("2026-07-12", "dueDate", "DefaultDueTime", "17:00").toISOString());"#;
    assert_eq!(run_js(MELBOURNE, snippet), "2026-07-12T07:00:00.000Z");
}

#[test]
fn write_date_uses_factory_default_when_setting_is_null() {
    let snippet = format!(
        r#"{}
print(parseWriteDate("2026-07-12", "dueDate", "DefaultDueTime", "17:00").toISOString());"#,
        fake_settings("null")
    );
    assert_eq!(run_js(MELBOURNE, &snippet), "2026-07-12T07:00:00.000Z");
}

#[test]
fn write_date_uses_factory_default_when_setting_is_undefined() {
    let snippet = format!(
        r#"{}
print(parseWriteDate("2026-07-12", "dueDate", "DefaultDueTime", "17:00").toISOString());"#,
        fake_settings("undefined")
    );
    assert_eq!(run_js(MELBOURNE, &snippet), "2026-07-12T07:00:00.000Z");
}

#[test]
fn write_date_uses_factory_default_when_setting_is_garbage() {
    let snippet = format!(
        r#"{}
print(parseWriteDate("2026-07-12", "dueDate", "DefaultDueTime", "17:00").toISOString());"#,
        fake_settings(r#""noon""#)
    );
    assert_eq!(run_js(MELBOURNE, &snippet), "2026-07-12T07:00:00.000Z");
}

#[test]
fn write_date_uses_factory_default_when_setting_hour_is_out_of_range() {
    let snippet = format!(
        r#"{}
print(parseWriteDate("2026-07-12", "dueDate", "DefaultDueTime", "17:00").toISOString());"#,
        fake_settings(r#""24:00""#)
    );
    assert_eq!(run_js(MELBOURNE, &snippet), "2026-07-12T07:00:00.000Z");
}

#[test]
fn write_date_uses_factory_default_when_setting_minute_is_out_of_range() {
    let snippet = format!(
        r#"{}
print(parseWriteDate("2026-07-12", "dueDate", "DefaultDueTime", "17:00").toISOString());"#,
        fake_settings(r#""12:60""#)
    );
    assert_eq!(run_js(MELBOURNE, &snippet), "2026-07-12T07:00:00.000Z");
}

#[test]
fn write_date_uses_factory_default_when_settings_lookup_throws() {
    let snippet = r#"var settings = { objectForKey: function () { throw new Error("no such key"); } };
print(parseWriteDate("2026-07-12", "dueDate", "DefaultDueTime", "17:00").toISOString());"#;
    assert_eq!(run_js(MELBOURNE, snippet), "2026-07-12T07:00:00.000Z");
}

#[test]
fn write_date_passes_non_bare_date_time_through_regardless_of_settings() {
    let snippet = format!(
        r#"{}
print(parseWriteDate("2026-07-12T09:30:00Z", "dueDate", "DefaultDueTime", "17:00").toISOString());
print(parseWriteDate("2026-07-12T09:30:00", "dueDate", "DefaultDueTime", "17:00").toISOString());"#,
        fake_settings(r#""08:30""#)
    );
    assert_eq!(
        run_js(MELBOURNE, &snippet),
        "2026-07-12T09:30:00.000Z\n2026-07-11T23:30:00.000Z"
    );
}

#[test]
fn write_date_rejects_impossible_calendar_day() {
    assert_eq!(
        error_of(
            MELBOURNE,
            r#"parseWriteDate("2026-02-30", "dueDate", "DefaultDueTime", "17:00")"#
        ),
        r#"ERROR: dueDate must be YYYY-MM-DD or an ISO 8601 date-time; received "2026-02-30""#
    );
}

#[test]
fn write_date_rejects_garbage() {
    assert_eq!(
        error_of(
            MELBOURNE,
            r#"parseWriteDate("soon", "deferDate", "DefaultStartTime", "00:00")"#
        ),
        r#"ERROR: deferDate must be YYYY-MM-DD or an ISO 8601 date-time; received "soon""#
    );
}

#[test]
fn write_date_rejects_invalid_factory_default_loudly() {
    assert_eq!(
        error_of(
            MELBOURNE,
            r#"parseWriteDate("2026-07-12", "dueDate", "DefaultDueTime", "5pm")"#
        ),
        r#"ERROR: invalid factory default time for DefaultDueTime: "5pm""#
    );
}
