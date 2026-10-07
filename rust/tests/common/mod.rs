//! JavaScriptCore helpers shared by the behavioural tests of OmniJS snippets.
//!
//! OmniFocus runs our scripts on JavaScriptCore. The macOS system shell `jsc`
//! is the same engine without OmniFocus around it, so a snippet can be run
//! against hand-built fakes of the OmniJS objects it reads (`Project.Status`,
//! a review interval, ...) without ever touching OmniFocus. Scripts are passed
//! with `jsc -e`, so no temporary files are written, and `TZ` is pinned to UTC
//! so nothing depends on the machine's time zone.

use std::{path::Path, process::Command};

const JSC_PATH: &str =
    "/System/Library/Frameworks/JavaScriptCore.framework/Versions/Current/Helpers/jsc";

/// Runs `prelude` followed by `snippet` in `jsc` and returns what the snippet
/// printed, trimmed.
///
/// Fails loudly when `jsc` is missing or the script throws: these tests must
/// never skip silently.
pub fn run_jsc(prelude: &str, snippet: &str) -> String {
    assert!(
        Path::new(JSC_PATH).exists(),
        "JavaScriptCore shell not found at {JSC_PATH}; these tests require macOS"
    );
    let script = format!("{prelude}\n{snippet}");
    let output = Command::new(JSC_PATH)
        .arg("-e")
        .arg(&script)
        .env("TZ", "UTC")
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
