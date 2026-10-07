//! Parsing of the `reviewInterval` text that update_project accepts
//! (upstream #12).
//!
//! OmniFocus stores a project's review interval as a `Project.ReviewInterval`
//! value with a whole-number `steps` count and a plural `unit` ("days",
//! "weeks", "months" or "years"). Users write it as "N unit", e.g. "2 weeks".
//! The text is validated here, in Rust, so a bad value is refused before any
//! script runs and therefore before any other field of the project changes.

use serde::Serialize;

use crate::error::{OmniFocusError, Result};

/// A validated review interval, shaped like OmniJS's `Project.ReviewInterval`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewInterval {
    /// How many `unit`s between reviews; at least 1.
    pub steps: u32,
    /// The plural unit name OmniFocus uses: days, weeks, months or years.
    pub unit: &'static str,
}

/// Accepted unit spellings (singular, plural) and the plural OmniFocus uses.
const UNITS: [(&str, &str); 4] = [
    ("day", "days"),
    ("week", "weeks"),
    ("month", "months"),
    ("year", "years"),
];

/// Parses "N unit": N a whole number of at least 1 written in ASCII digits,
/// unit one of day(s), week(s), month(s), year(s) in any letter case. Leading,
/// trailing and repeated inner whitespace are ignored.
///
/// Anything else, including minutes, hours or ISO 8601 durations such as
/// "P1W", is a `Validation` error quoting the received text.
pub fn parse_review_interval(input: &str) -> Result<ReviewInterval> {
    let invalid = || {
        OmniFocusError::Validation(format!(
            "reviewInterval must be \"N unit\", where N is a whole number of at least 1 \
             and unit is one of days, weeks, months, years (for example \"2 weeks\"); \
             received {input:?}."
        ))
    };

    let mut words = input.split_whitespace();
    let (Some(steps_text), Some(unit_text), None) = (words.next(), words.next(), words.next())
    else {
        return Err(invalid());
    };

    // `str::parse::<u32>` would also accept a leading "+".
    if !steps_text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid());
    }
    let steps: u32 = steps_text.parse().map_err(|_| invalid())?;
    if steps < 1 {
        return Err(invalid());
    }

    let unit_text = unit_text.to_ascii_lowercase();
    let unit = UNITS
        .iter()
        .find(|(singular, plural)| unit_text == *singular || unit_text == *plural)
        .map(|(_, plural)| *plural)
        .ok_or_else(invalid)?;

    Ok(ReviewInterval { steps, unit })
}
