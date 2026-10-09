//! Runs Omni Automation (OmniJS) scripts in OmniFocus.
//!
//! A script is wrapped twice: inside OmniFocus it runs in a function whose
//! result or exception becomes a JSON `{ok, data}` / `{ok: false, error}`
//! envelope, and that wrapper is handed to OmniFocus's `evaluateJavascript`
//! from a JXA script. The JXA script is run by a program, osascript in
//! production, described by `JxaProcess`. One call runs at a time.

use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
    sync::OnceLock,
    time::Duration,
};

use serde_json::Value;
use tokio::{process::Command, sync::Mutex, time::timeout};

use crate::error::{OmniFocusError, Result};

/// The program that runs JXA scripts in production, by absolute path: looked
/// up through PATH, a user-writable directory listed before /usr/bin would
/// choose the program that receives every script.
pub const OSASCRIPT: &str = "/usr/bin/osascript";
/// How long one call may take in production.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
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

/// The program that runs JXA scripts and how long one call may take.
///
/// Production uses `JxaProcess::osascript()`. Tests put a stub program in its
/// place with `JxaProcess::new`, so the process handling can be exercised
/// without OmniFocus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JxaProcess {
    program: PathBuf,
    time_limit: Duration,
}

impl JxaProcess {
    /// osascript with the 30 s limit.
    pub fn osascript() -> Self {
        Self::new(OSASCRIPT, DEFAULT_TIMEOUT)
    }

    pub fn new(program: impl Into<PathBuf>, time_limit: Duration) -> Self {
        Self {
            program: program.into(),
            time_limit,
        }
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    pub fn time_limit(&self) -> Duration {
        self.time_limit
    }

    /// Runs a JXA script and returns its trimmed stdout.
    ///
    /// Waits for any call already running, then gives the program
    /// `time_limit` to finish; past it the program is killed and the call
    /// fails with `Timeout`. A non-zero exit fails with osascript's stderr
    /// read by `friendly_jxa_error`.
    pub async fn run(&self, script: &str) -> Result<String> {
        let _guard = jxa_call_lock().lock().await;
        let child = Command::new(&self.program)
            .arg("-l")
            .arg("JavaScript")
            .arg("-e")
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let output = match timeout(self.time_limit, child.wait_with_output()).await {
            Ok(output) => output?,
            Err(_) => {
                return Err(OmniFocusError::Timeout {
                    after: self.time_limit,
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

    /// Runs a JXA script and parses its stdout as JSON.
    pub async fn run_json(&self, script: &str) -> Result<Value> {
        let stdout = self.run(script).await?;
        parse_jxa_output(&stdout)
    }

    /// Runs an OmniJS script inside OmniFocus and returns its result, or the
    /// error it raised.
    pub async fn run_omnijs(&self, script: &str) -> Result<Value> {
        let envelope = self.run_json(&omnijs_jxa_script(script)).await?;
        unwrap_omnijs_envelope(envelope)
    }
}

pub async fn run_jxa(script: &str) -> Result<String> {
    JxaProcess::osascript().run(script).await
}

pub async fn run_jxa_with_timeout(script: &str, time_limit: Duration) -> Result<String> {
    JxaProcess::new(OSASCRIPT, time_limit).run(script).await
}

pub async fn run_jxa_json(script: &str) -> Result<Value> {
    JxaProcess::osascript().run_json(script).await
}

pub async fn run_jxa_json_with_timeout(script: &str, time_limit: Duration) -> Result<Value> {
    JxaProcess::new(OSASCRIPT, time_limit)
        .run_json(script)
        .await
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
    JxaProcess::osascript().run_omnijs(script).await
}

pub async fn run_omnijs_with_timeout(script: &str, time_limit: Duration) -> Result<Value> {
    JxaProcess::new(OSASCRIPT, time_limit)
        .run_omnijs(script)
        .await
}

/// The JXA script that runs `script` inside OmniFocus: the OmniJS wrapper
/// that returns the `{ok, data}` / `{ok: false, error}` envelope as a JSON
/// string, passed to `evaluateJavascript`.
pub fn omnijs_jxa_script(script: &str) -> String {
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

    format!(
        "const app = Application('OmniFocus');\nconst result = app.evaluateJavascript({});\nresult;",
        escape_for_jxa(&wrapped_omnijs)
    )
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

/// Runs scripts in OmniFocus through a `JxaProcess`: osascript with the 30 s
/// limit unless built with `with_process`.
#[derive(Debug, Clone)]
pub struct RealJxaRunner {
    process: JxaProcess,
}

impl RealJxaRunner {
    pub fn new() -> Self {
        Self::with_process(JxaProcess::osascript())
    }

    pub fn with_process(process: JxaProcess) -> Self {
        Self { process }
    }

    pub fn process(&self) -> &JxaProcess {
        &self.process
    }
}

impl Default for RealJxaRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl JxaRunner for RealJxaRunner {
    fn run_omnijs<'a>(
        &'a self,
        script: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + 'a>> {
        Box::pin(self.process.run_omnijs(script))
    }
}

pub async fn run_script_with_runner<R: JxaRunner>(runner: &R, script: &str) -> Result<Value> {
    runner.run_omnijs(script).await
}
