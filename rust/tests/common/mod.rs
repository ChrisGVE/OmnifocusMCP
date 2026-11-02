//! JavaScriptCore helpers shared by the behavioural tests of OmniJS snippets.
//!
//! OmniFocus runs our scripts on JavaScriptCore. The macOS system shell `jsc`
//! is the same engine without OmniFocus around it, so a snippet can be run
//! against hand-built fakes of the OmniJS objects it reads (`Project.Status`,
//! a review interval, ...) without ever touching OmniFocus. Scripts are passed
//! with `jsc -e`, so no temporary files are written. `TZ` is always set
//! explicitly, so nothing depends on the machine's time zone: `run_jsc` uses
//! UTC, and `run_jsc_in_time_zone` takes the zone for tests that must see
//! local-time behaviour (a DST change, a day boundary away from UTC).

use std::{path::Path, process::Command};

const JSC_PATH: &str =
    "/System/Library/Frameworks/JavaScriptCore.framework/Versions/Current/Helpers/jsc";

/// The default time zone of the jsc tests.
pub const UTC: &str = "UTC";

/// A `jsc` command running in `time_zone` (an IANA name such as
/// `"Europe/Zurich"`). Panics when `jsc` is missing: these tests must never
/// skip silently.
pub fn jsc_command(time_zone: &str) -> Command {
    assert!(
        Path::new(JSC_PATH).exists(),
        "JavaScriptCore shell not found at {JSC_PATH}; these tests require macOS"
    );
    let mut command = Command::new(JSC_PATH);
    command.env("TZ", time_zone);
    command
}

/// Runs `prelude` followed by `snippet` in `jsc`, in UTC, and returns what
/// the snippet printed, trimmed. See `run_jsc_in_time_zone`.
pub fn run_jsc(prelude: &str, snippet: &str) -> String {
    run_jsc_in_time_zone(UTC, prelude, snippet)
}

/// Runs `prelude` followed by `snippet` in `jsc` with `TZ` set to
/// `time_zone`, and returns what the snippet printed, trimmed.
///
/// Fails loudly when `jsc` is missing or the script throws: these tests must
/// never skip silently.
pub fn run_jsc_in_time_zone(time_zone: &str, prelude: &str, snippet: &str) -> String {
    let script = format!("{prelude}\n{snippet}");
    let output = jsc_command(time_zone)
        .arg("-e")
        .arg(&script)
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

/// Asserts that a complete tool script is syntactically valid JavaScript.
///
/// Tool scripts end in a top-level `return` because `crate::jxa` wraps them in
/// a function, so the check compiles the script as a function body without
/// running it. This catches what text assertions cannot, such as a shared
/// helper prepended next to a leftover local definition of the same name.
pub fn assert_script_compiles(tool: &str, script: &str) {
    let literal = serde_json::to_string(script).expect("script serializes as a JS string");
    let verdict = run_jsc(
        "",
        &format!(
            "try {{ new Function({literal}); print(\"COMPILES\"); }} \
             catch (error) {{ print(\"SYNTAX ERROR: \" + error.message); }}"
        ),
    );
    assert_eq!(verdict, "COMPILES", "{tool} script must compile");
}
