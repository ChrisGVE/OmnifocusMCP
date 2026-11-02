//! A `JxaRunner` that runs each tool script the way production does, minus
//! OmniFocus: through the real `osascript` program built by
//! `omnifocus_mcp::jxa::omnijs_jxa_script`, in JavaScriptCore, against
//! the strict OmniJS fake in `fixtures/omnijs/`.
//!
//! Two JavaScriptCore realms stand in for the two layers of a real call:
//!
//! - the outer realm plays osascript's JXA: it defines `Application`, whose
//!   `evaluateJavascript` hands the wrapped script to the inner realm, and it
//!   evaluates the production program as osascript would;
//! - the inner realm plays OmniFocus's OmniJS context: the documented
//!   interface (`interface.js`), the strict fake (`fake/*.js`) and the
//!   scenario database (`scenario.js`).
//!
//! Nothing here can reach OmniFocus: `jsc` is a bare JavaScript engine, and
//! `Application` is a function of this file.
//!
//! The runner returns what `crate::jxa::run_omnijs` would: the envelope's
//! data, or an error. A script that touched an undocumented member, or a
//! member the fake does not model, fails even if it caught the throw. Every
//! run is also recorded, so a test can check what was compiled and run.
//!
//! Test files include it with `#[path = "support/omnijs_harness.rs"] mod
//! omnijs_harness;`, next to `mod common;`.

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    error::{OmniFocusError, Result},
    jxa::{omnijs_jxa_script, parse_jxa_output, unwrap_omnijs_envelope, JxaRunner},
};
use serde::Deserialize;
use serde_json::Value;

use crate::common::jsc_command;

const INTERFACE_JS: &str = include_str!("../fixtures/omnijs/interface.js");
/// The strict fake, in load order: each file uses what the earlier ones
/// define, and the last one removes the namespace they share.
const FAKE_JS: [&str; 11] = [
    include_str!("../fixtures/omnijs/fake/strict.js"),
    include_str!("../fixtures/omnijs/fake/hosts.js"),
    include_str!("../fixtures/omnijs/fake/store.js"),
    include_str!("../fixtures/omnijs/fake/task_rules.js"),
    include_str!("../fixtures/omnijs/fake/tasks.js"),
    include_str!("../fixtures/omnijs/fake/notifications.js"),
    include_str!("../fixtures/omnijs/fake/projects.js"),
    include_str!("../fixtures/omnijs/fake/folders.js"),
    include_str!("../fixtures/omnijs/fake/tags.js"),
    include_str!("../fixtures/omnijs/fake/constructors.js"),
    include_str!("../fixtures/omnijs/fake/globals.js"),
];
const SCENARIO_JS: &str = include_str!("../fixtures/omnijs/scenario.js");

/// The instant `new Date()` returns inside the fake, so results do not
/// depend on when the tests run.
pub const FIXED_NOW: &str = "2026-10-09T10:00:00Z";

/// Marks the line on which the harness prints its report, apart from
/// anything a script itself prints.
const REPORT_MARKER: &str = "__OMNIJS_HARNESS_REPORT__";

/// What happened to one script.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptRun {
    /// The production osascript program compiled.
    pub outer_compiled: bool,
    /// The wrapped OmniJS script compiled in the OmniJS realm.
    pub inner_compiled: bool,
    /// `evaluateJavascript` ran the wrapped script and returned its envelope.
    pub executed: bool,
    /// The syntax or runtime error that stopped the run, if any.
    pub error: Option<String>,
    /// What osascript would have printed: the envelope as JSON text.
    pub output: Option<String>,
    /// Undocumented members the script read or wrote.
    pub violations: Vec<String>,
    /// Documented members the fake does not model.
    pub gaps: Vec<String>,
    /// `new Date(2026, 0, 1).getTimezoneOffset()` inside the OmniJS realm,
    /// which proves which time zone the script ran in.
    pub january_offset_minutes: i64,
}

/// The harness runner. Clones share one log of runs.
#[derive(Clone)]
pub struct StrictOmniJs {
    time_zone: &'static str,
    runs: Arc<Mutex<Vec<ScriptRun>>>,
}

impl StrictOmniJs {
    /// A runner whose scripts run with `TZ` set to `time_zone`.
    pub fn new(time_zone: &'static str) -> Self {
        Self {
            time_zone,
            runs: Arc::default(),
        }
    }

    /// Every script run so far, in order.
    pub fn runs(&self) -> Vec<ScriptRun> {
        self.runs.lock().expect("run log lock").clone()
    }

    async fn run(&self, script: &str) -> Result<Value> {
        let program = harness_program(&omnijs_jxa_script(script));
        let mut command = tokio::process::Command::from(jsc_command(self.time_zone));
        let output = command
            .arg("-e")
            .arg(&program)
            .kill_on_drop(true)
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let report = stdout
            .lines()
            .find_map(|line| line.strip_prefix(REPORT_MARKER))
            .ok_or_else(|| {
                OmniFocusError::JxaExecution(format!(
                    "harness: jsc printed no report (exit {}): {}{}",
                    output.status,
                    stdout,
                    String::from_utf8_lossy(&output.stderr)
                ))
            })?;
        let run: ScriptRun =
            serde_json::from_str(report).map_err(OmniFocusError::MalformedOutput)?;
        self.runs.lock().expect("run log lock").push(run.clone());
        outcome(&run)
    }
}

/// The result `crate::jxa::run_omnijs` would have returned for `run`.
fn outcome(run: &ScriptRun) -> Result<Value> {
    if !run.violations.is_empty() || !run.gaps.is_empty() {
        return Err(OmniFocusError::OmniFocus(format!(
            "strict OmniJS fake: violations {:?}, unmodelled {:?}",
            run.violations, run.gaps
        )));
    }
    match (&run.output, &run.error) {
        (Some(output), None) => unwrap_omnijs_envelope(parse_jxa_output(output.trim())?),
        (_, error) => Err(OmniFocusError::JxaExecution(
            error
                .clone()
                .unwrap_or_else(|| "harness: no output".to_string()),
        )),
    }
}

impl JxaRunner for StrictOmniJs {
    fn run_omnijs<'a>(
        &'a self,
        script: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + 'a>> {
        Box::pin(self.run(script))
    }
}

fn js_string(text: &str) -> String {
    serde_json::to_string(text).expect("a string serializes")
}

/// The jsc program for one run: set up the OmniJS realm, define the JXA
/// `Application`, evaluate the production osascript program, print a report.
fn harness_program(osascript_program: &str) -> String {
    let config = js_string(&format!(
        "globalThis.__HARNESS_CONFIG = {{ now: {} }};",
        js_string(FIXED_NOW)
    ));
    format!(
        r#"const omni = $262.createRealm();
omni.evalScript({interface});
omni.evalScript({config});
{fake}
omni.evalScript({scenario});
const report = {{
  outerCompiled: false, innerCompiled: false, executed: false,
  error: null, output: null, violations: [], gaps: [],
  januaryOffsetMinutes: omni.evalScript("new Date(2026, 0, 1).getTimezoneOffset()"),
}};
function describeError(error) {{
  return (error && error.name ? error.name + ": " : "") + (error && error.message ? error.message : String(error));
}}
var Application = function (name) {{
  if (name !== "OmniFocus") throw new Error("harness: unexpected Application(" + name + ")");
  return {{
    evaluateJavascript(source) {{
      new omni.global.Function(source);
      report.innerCompiled = true;
      const envelope = omni.evalScript(source);
      report.executed = true;
      return envelope;
    }},
  }};
}};
const program = {program};
try {{
  new Function(program);
  report.outerCompiled = true;
  report.output = String((0, eval)(program));
}} catch (error) {{
  report.error = describeError(error);
}}
report.violations = Array.from(omni.global.__harness.violations);
report.gaps = Array.from(omni.global.__harness.gaps);
print({marker} + JSON.stringify(report));
"#,
        interface = js_string(INTERFACE_JS),
        fake = FAKE_JS
            .iter()
            .map(|source| format!("omni.evalScript({});", js_string(source)))
            .collect::<Vec<_>>()
            .join("\n"),
        scenario = js_string(SCENARIO_JS),
        program = js_string(osascript_program),
        marker = js_string(REPORT_MARKER),
    )
}
