//! How a JXA call's time limit is taken and reported (audit CR-044).
//!
//! The limit used to be an `f64` number of seconds handed to
//! `Duration::from_secs_f64`, which panics on a negative, NaN or overflowing
//! value, and the timeout message printed it with `{:.0}`, so half a second
//! read "timed out after 0s". The limit is now a `Duration`, which cannot hold
//! those values, and the message names it in the unit that reads naturally.

use std::time::Duration;

use omnifocus_mcp::{
    error::{describe_duration, OmniFocusError},
    jxa::{run_jxa_json_with_timeout, run_jxa_with_timeout, run_omnijs_with_timeout},
};

// ------------------------------------------------ the limit is a Duration

/// Compiles only while the public runners take a `Duration`. The closures
/// are never called, so nothing is run.
#[test]
fn public_runners_take_a_duration() {
    let limit = Duration::from_secs(1);
    let _raw = |script: &'static str| run_jxa_with_timeout(script, limit);
    let _json = |script: &'static str| run_jxa_json_with_timeout(script, limit);
    let _omnijs = |script: &'static str| run_omnijs_with_timeout(script, limit);
}

// ------------------------------------------------ describing a duration

#[test]
fn whole_seconds_read_as_seconds() {
    assert_eq!(describe_duration(Duration::from_secs(30)), "30s");
    assert_eq!(describe_duration(Duration::from_secs(1)), "1s");
}

#[test]
fn fractional_seconds_keep_their_fraction() {
    assert_eq!(describe_duration(Duration::from_millis(1500)), "1.5s");
    assert_eq!(describe_duration(Duration::from_millis(2250)), "2.25s");
}

#[test]
fn under_a_second_reads_as_milliseconds() {
    assert_eq!(describe_duration(Duration::from_millis(500)), "500ms");
    assert_eq!(describe_duration(Duration::from_millis(1)), "1ms");
}

#[test]
fn under_a_millisecond_reads_as_microseconds() {
    assert_eq!(describe_duration(Duration::from_micros(250)), "250µs");
}

#[test]
fn under_a_microsecond_reads_as_nanoseconds() {
    assert_eq!(describe_duration(Duration::from_nanos(7)), "7ns");
}

#[test]
fn zero_reads_as_zero_seconds() {
    assert_eq!(describe_duration(Duration::ZERO), "0s");
}

// ------------------------------------------------ the timeout error

#[test]
fn sub_second_timeout_is_not_reported_as_zero_seconds() {
    let error = OmniFocusError::Timeout {
        after: Duration::from_millis(500),
    };
    let message = error.to_string();
    assert!(message.contains("500ms"), "{message}");
    assert!(!message.contains("0s"), "{message}");
}

#[test]
fn timeout_error_keeps_its_duration() {
    let error = OmniFocusError::Timeout {
        after: Duration::from_secs(30),
    };
    assert!(
        matches!(error, OmniFocusError::Timeout { after } if after == Duration::from_secs(30)),
        "{error:?}"
    );
}
