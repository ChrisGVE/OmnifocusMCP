use std::time::Duration;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, OmniFocusError>;

#[derive(Error, Debug)]
pub enum OmniFocusError {
    #[error("JXA execution failed: {0}")]
    JxaExecution(String),
    #[error("{0}")]
    OmniFocus(String),
    /// osascript's stdout was not JSON at all.
    #[error("JXA command returned malformed JSON: {0}")]
    MalformedOutput(serde_json::Error),
    /// The script's JSON parsed, but not into the shape the tool reads it as
    /// (a missing field, a value of the wrong type). Kept apart from
    /// `MalformedOutput` because the cause is a script and its reader
    /// disagreeing, not broken output.
    #[error("OmniFocus returned a result in an unexpected shape: {0}")]
    UnexpectedResultShape(serde_json::Error),
    /// The server could not encode its own data (script input, a prompt or
    /// resource body) as JSON.
    #[error("Could not encode JSON: {0}")]
    Encoding(serde_json::Error),
    #[error("{0}")]
    Validation(String),
    #[error("I/O error while running JXA: {0}")]
    Io(#[from] std::io::Error),
    #[error("JXA command timed out after {}.", describe_duration(*.after))]
    Timeout { after: Duration },
}

/// Names a duration in the largest unit it fills at least once (s, ms, µs,
/// ns), with up to three decimals and no trailing zeros: `30s`, `1.5s`,
/// `500ms`. Zero reads `0s`.
pub fn describe_duration(duration: Duration) -> String {
    const UNITS: [(u128, &str); 4] = [
        (1_000_000_000, "s"),
        (1_000_000, "ms"),
        (1_000, "µs"),
        (1, "ns"),
    ];
    let nanos = duration.as_nanos();
    if nanos == 0 {
        return "0s".to_string();
    }
    let (unit_nanos, suffix) = UNITS
        .into_iter()
        .find(|(unit_nanos, _)| nanos >= *unit_nanos)
        .unwrap_or((1, "ns"));
    let value = format!("{:.3}", nanos as f64 / unit_nanos as f64);
    let value = value.trim_end_matches('0').trim_end_matches('.');
    format!("{value}{suffix}")
}

/// Reads a script's JSON result into the type a tool returns.
///
/// Every tool that deserializes a script result goes through here, so a
/// mismatch is always reported as `UnexpectedResultShape` with serde's
/// explanation of which field was wrong. There is deliberately no
/// `From<serde_json::Error>`: a bare `?` could not tell broken output from a
/// shape mismatch, which is how the two came to share one message.
pub fn from_result_value<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Result<T> {
    serde_json::from_value(value).map_err(OmniFocusError::UnexpectedResultShape)
}

/// Encodes the server's own data as a JSON string, reporting a failure as
/// `Encoding`.
pub fn to_json_string<T: serde::Serialize + ?Sized>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(OmniFocusError::Encoding)
}
