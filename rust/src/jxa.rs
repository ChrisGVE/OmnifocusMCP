use std::{future::Future, pin::Pin, sync::OnceLock, time::Duration};

use serde_json::Value;
use tokio::{process::Command, sync::Mutex, time::timeout};

use crate::error::{OmniFocusError, Result};

const DEFAULT_TIMEOUT_SECONDS: f64 = 30.0;
static JXA_CALL_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn jxa_call_lock() -> &'static Mutex<()> {
    JXA_CALL_LOCK.get_or_init(|| Mutex::new(()))
}

pub fn escape_for_jxa(value: &str) -> String {
    match serde_json::to_string(value) {
        Ok(escaped) => escaped,
        Err(_) => "\"\"".to_string(),
    }
}

/// Turns osascript's stderr, after a non-zero exit, into the message a user
/// can act on: OmniFocus not running, Automation access refused, or a script
/// syntax error; anything else is returned trimmed.
///
/// This is the only classifier, and it only ever sees osascript's own stderr.
/// A message our script raises inside OmniFocus comes back in the
/// `{ok: false, error}` envelope instead and is passed through unchanged by
/// `unwrap_omnijs_envelope`: OmniFocus was running and reachable for the
/// script to run at all, and the message often echoes user input such as a
/// tag name, which these substring checks would misread.
pub fn friendly_jxa_error(stderr: &str) -> String {
    let lowered = stderr.to_lowercase();
    let names_omnifocus = lowered.contains("omnifocus");
    if names_omnifocus
        && (lowered.contains("not running") || lowered.contains("application isn't running"))
    {
        return "OmniFocus is not running. Please open OmniFocus and try again.".to_string();
    }
    if lowered.contains("not authorized")
        || lowered.contains("not permitted")
        || lowered.contains("not authorised")
        || lowered.contains("apple events")
        || lowered.contains("(-1743)")
    {
        return "macOS blocked Automation access to OmniFocus. Grant permission in System Settings > Privacy & Security > Automation.".to_string();
    }
    if lowered.contains("syntax error") {
        return format!("JXA script syntax error: {}", stderr.trim());
    }
    stderr.trim().to_string()
}

pub async fn run_jxa(script: &str) -> Result<String> {
    run_jxa_with_timeout(script, DEFAULT_TIMEOUT_SECONDS).await
}

pub async fn run_jxa_with_timeout(script: &str, timeout_seconds: f64) -> Result<String> {
    let _guard = jxa_call_lock().lock().await;
    let duration = Duration::from_secs_f64(timeout_seconds);

    let child = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(script)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let output = match timeout(duration, child.wait_with_output()).await {
        Ok(output) => output?,
        Err(_) => {
            return Err(OmniFocusError::Timeout {
                seconds: timeout_seconds,
            });
        }
    };

    if !output.status.success() {
        let stderr_text = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(OmniFocusError::JxaExecution(friendly_jxa_error(
            &stderr_text,
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub async fn run_jxa_json(script: &str) -> Result<Value> {
    run_jxa_json_with_timeout(script, DEFAULT_TIMEOUT_SECONDS).await
}

pub async fn run_jxa_json_with_timeout(script: &str, timeout_seconds: f64) -> Result<Value> {
    let stdout = run_jxa_with_timeout(script, timeout_seconds).await?;
    parse_jxa_output(&stdout)
}

/// Parses osascript's trimmed stdout as JSON. Output that is not JSON is
/// `MalformedOutput`, carrying serde's explanation.
pub fn parse_jxa_output(stdout: &str) -> Result<Value> {
    if stdout.is_empty() {
        return Err(OmniFocusError::JxaExecution(
            "JXA command returned empty output.".to_string(),
        ));
    }
    serde_json::from_str::<Value>(stdout).map_err(OmniFocusError::MalformedOutput)
}

pub async fn run_omnijs(script: &str) -> Result<Value> {
    run_omnijs_with_timeout(script, DEFAULT_TIMEOUT_SECONDS).await
}

pub async fn run_omnijs_with_timeout(script: &str, timeout_seconds: f64) -> Result<Value> {
    let wrapped_omnijs = format!(
        r#"(function() {{
  try {{
    if (typeof document === "object" && document) {{
      if (typeof document.flattenedTasks === "undefined" && typeof flattenedTasks !== "undefined") {{
        document.flattenedTasks = flattenedTasks;
      }}
      if (
        typeof document.flattenedProjects === "undefined"
        && typeof flattenedProjects !== "undefined"
      ) {{
        document.flattenedProjects = flattenedProjects;
      }}
      if (typeof document.flattenedTags === "undefined" && typeof flattenedTags !== "undefined") {{
        document.flattenedTags = flattenedTags;
      }}
      if (
        typeof document.flattenedFolders === "undefined"
        && typeof flattenedFolders !== "undefined"
      ) {{
        document.flattenedFolders = flattenedFolders;
      }}
    }}
    const __data = (function() {{
{script}
    }})();
    return JSON.stringify({{ ok: true, data: __data }});
  }} catch (e) {{
    return JSON.stringify({{ ok: false, error: e && e.message ? e.message : String(e) }});
  }}
}})()"#
    );

    let outer_jxa = format!(
        "const app = Application('OmniFocus');\nconst result = app.evaluateJavascript({});\nresult;",
        escape_for_jxa(&wrapped_omnijs)
    );

    let envelope = run_jxa_json_with_timeout(&outer_jxa, timeout_seconds).await?;
    unwrap_omnijs_envelope(envelope)
}

/// Reads the `{ok, data}` / `{ok: false, error}` envelope our OmniJS wrapper
/// returns. A script error's message is returned as is (trimmed), whatever it
/// says; see `friendly_jxa_error` for why it is not classified.
pub fn unwrap_omnijs_envelope(envelope: Value) -> Result<Value> {
    let envelope_obj = envelope.as_object().ok_or_else(|| {
        OmniFocusError::OmniFocus("OmniFocus returned an unexpected response.".to_string())
    })?;

    if envelope_obj.get("ok").and_then(Value::as_bool) != Some(true) {
        if let Some(error) = envelope_obj.get("error").and_then(Value::as_str) {
            let cleaned = error.trim();
            if !cleaned.is_empty() {
                return Err(OmniFocusError::OmniFocus(cleaned.to_string()));
            }
        }
        return Err(OmniFocusError::OmniFocus(
            "OmniFocus script error.".to_string(),
        ));
    }

    Ok(envelope_obj.get("data").cloned().unwrap_or(Value::Null))
}

pub trait JxaRunner: Send + Sync {
    fn run_omnijs<'a>(
        &'a self,
        script: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + 'a>>;
}

#[derive(Debug, Clone, Default)]
pub struct RealJxaRunner;

impl RealJxaRunner {
    pub fn new() -> Self {
        Self
    }
}

impl JxaRunner for RealJxaRunner {
    fn run_omnijs<'a>(
        &'a self,
        script: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + 'a>> {
        Box::pin(async move { run_omnijs(script).await })
    }
}

pub async fn run_script_with_runner<R: JxaRunner>(runner: &R, script: &str) -> Result<Value> {
    runner.run_omnijs(script).await
}
