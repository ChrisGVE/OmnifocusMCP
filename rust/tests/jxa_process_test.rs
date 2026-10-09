//! The process layer of the JXA runner, driven through a stub program
//! (audit CR-029).
//!
//! `RealJxaRunner` starts a program, hands it a script, waits for it within a
//! time limit and reads its exit status, stderr and stdout. In production the
//! program is osascript, which drives OmniFocus, so this layer used to have no
//! test at all. `JxaProcess` names the program and the limit, and these tests
//! put a small shell script in osascript's place: each stub plays one way the
//! real program can end, and the test pins what the runner makes of it. Only
//! the stub is ever run; nothing here starts osascript or reaches OmniFocus.
//!
//! The stubs live in a fresh directory under Cargo's per-target temporary
//! directory, one per test.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use omnifocus_mcp::{
    error::OmniFocusError,
    jxa::{omnijs_jxa_script, JxaProcess, JxaRunner, RealJxaRunner},
};
use serde_json::{json, Value};

const NOT_RUNNING_MESSAGE: &str = "OmniFocus is not running. Please open OmniFocus and try again.";

/// Long enough that no stub below comes near it unless it is meant to.
const GENEROUS_LIMIT: Duration = Duration::from_secs(20);

/// A shell script standing in for osascript, and the directory it may write
/// its observations to.
struct Stub {
    dir: PathBuf,
    program: PathBuf,
}

/// Writes an executable `/bin/sh` script with `body` into a fresh directory
/// named after the test. `$STUB_DIR` in the body is that directory.
fn stub(test_name: &str, body: &str) -> Stub {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("jxa-process-stubs")
        .join(test_name);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the stub directory left by an earlier run");
    }
    fs::create_dir_all(&dir).expect("create the stub directory");
    let program = dir.join("osascript-stub");
    let script = format!("#!/bin/sh\nSTUB_DIR='{}'\n{body}\n", dir.display());
    fs::write(&program, script).expect("write the stub program");
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
        .expect("make the stub executable");
    Stub { dir, program }
}

fn runner(stub: &Stub, time_limit: Duration) -> RealJxaRunner {
    RealJxaRunner::with_process(JxaProcess::new(&stub.program, time_limit))
}

/// Runs a trivial OmniJS script through a stub and returns the outcome.
async fn run_through(stub: &Stub) -> omnifocus_mcp::error::Result<Value> {
    runner(stub, GENEROUS_LIMIT).run_omnijs("return 1;").await
}

/// Runs through a stub that reads its input, prints `stdout` and exits 0.
async fn outcome_of_stdout(test_name: &str, stdout: &str) -> omnifocus_mcp::error::Result<Value> {
    let body = format!("cat > /dev/null\nprintf '%s' '{stdout}'\nexit 0");
    run_through(&stub(test_name, &body)).await
}

// ------------------------------------------------ production configuration

#[test]
fn production_process_keeps_the_thirty_second_limit() {
    assert_eq!(
        JxaProcess::osascript().time_limit(),
        Duration::from_secs(30)
    );
}

#[test]
fn a_process_reports_the_program_and_limit_it_was_given() {
    let process = JxaProcess::new("/some/program", Duration::from_millis(1500));
    assert_eq!(process.program(), std::path::Path::new("/some/program"));
    assert_eq!(process.time_limit(), Duration::from_millis(1500));
}

// ------------------------------------------------ the program never answers

#[tokio::test]
async fn a_call_that_outlives_the_limit_is_a_timeout_and_the_program_is_killed() {
    let stub = stub(
        "timeout",
        "cat > /dev/null\necho $$ > \"$STUB_DIR/pid\"\nexec sleep 30",
    );
    let limit = Duration::from_millis(300);
    let started = Instant::now();

    let error = runner(&stub, limit)
        .run_omnijs("return 1;")
        .await
        .expect_err("a stub that sleeps past the limit must time out");

    assert!(
        matches!(error, OmniFocusError::Timeout { after } if after == limit),
        "{error:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "the runner waited {:?} instead of giving up at the limit",
        started.elapsed()
    );
    let pid = fs::read_to_string(stub.dir.join("pid")).expect("the stub recorded its pid");
    assert_process_ends(pid.trim());
}

/// Waits up to five seconds for `pid` to be gone or a zombie. Uses `ps`,
/// which only reads the process table.
fn assert_process_ends(pid: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let output = Command::new("/bin/ps")
            .args(["-o", "stat=", "-p", pid])
            .output()
            .expect("run ps");
        let state = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if state.is_empty() || state.starts_with('Z') {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "process {pid} still running ({state}) after the timeout"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

// ------------------------------------------------ the program fails

#[tokio::test]
async fn non_zero_exit_reports_stderr_through_the_classifier() {
    let stub = stub(
        "nonzero-known",
        "cat > /dev/null\n\
         echo 'execution error: OmniFocus got an error: Application isn'\"'\"'t running. (-600)' >&2\n\
         exit 1",
    );
    let error = run_through(&stub).await.expect_err("exit 1 is a failure");
    assert!(
        matches!(&error, OmniFocusError::JxaExecution(message) if message == NOT_RUNNING_MESSAGE),
        "{error:?}"
    );
}

#[tokio::test]
async fn non_zero_exit_with_unrecognised_stderr_passes_it_trimmed() {
    let stub = stub(
        "nonzero-unknown",
        "cat > /dev/null\nprintf '  something broke  \\n' >&2\nexit 3",
    );
    let error = run_through(&stub).await.expect_err("exit 3 is a failure");
    assert!(
        matches!(&error, OmniFocusError::JxaExecution(message) if message == "something broke"),
        "{error:?}"
    );
}

#[tokio::test]
async fn non_zero_exit_wins_over_json_on_stdout() {
    let stub = stub(
        "nonzero-with-stdout",
        "cat > /dev/null\nprintf '{\"ok\":true,\"data\":1}'\necho 'boom' >&2\nexit 1",
    );
    let error = run_through(&stub).await.expect_err("exit 1 is a failure");
    assert!(
        matches!(&error, OmniFocusError::JxaExecution(message) if message == "boom"),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_program_that_cannot_be_started_is_an_io_error() {
    let missing = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("jxa-process-stubs")
        .join("no-such-program");
    let runner = RealJxaRunner::with_process(JxaProcess::new(missing, GENEROUS_LIMIT));
    let error = runner
        .run_omnijs("return 1;")
        .await
        .expect_err("a missing program cannot run");
    assert!(matches!(error, OmniFocusError::Io(_)), "{error:?}");
}

// ------------------------------------------------ the program answers oddly

#[tokio::test]
async fn empty_stdout_is_an_execution_error() {
    let error = outcome_of_stdout("empty-stdout", "")
        .await
        .expect_err("no output is a failure");
    assert!(
        matches!(&error, OmniFocusError::JxaExecution(message)
            if message == "JXA command returned empty output."),
        "{error:?}"
    );
}

#[tokio::test]
async fn whitespace_only_stdout_is_an_execution_error() {
    let error = outcome_of_stdout("blank-stdout", "  \n\n ")
        .await
        .expect_err("blank output is a failure");
    assert!(
        matches!(&error, OmniFocusError::JxaExecution(message)
            if message == "JXA command returned empty output."),
        "{error:?}"
    );
}

#[tokio::test]
async fn non_json_stdout_is_malformed_output() {
    let error = outcome_of_stdout("non-json", "hello from osascript")
        .await
        .expect_err("text that is not JSON is a failure");
    assert!(
        matches!(error, OmniFocusError::MalformedOutput(_)),
        "{error:?}"
    );
    assert!(
        error
            .to_string()
            .starts_with("JXA command returned malformed JSON: "),
        "{error}"
    );
}

#[tokio::test]
async fn json_that_is_not_an_envelope_object_is_an_unexpected_response() {
    let error = outcome_of_stdout("not-an-object", "[1, 2]")
        .await
        .expect_err("an array is not an envelope");
    assert!(
        matches!(&error, OmniFocusError::OmniFocus(message)
            if message == "OmniFocus returned an unexpected response."),
        "{error:?}"
    );
}

#[tokio::test]
async fn failed_envelope_with_empty_error_reads_as_a_script_error() {
    let error = outcome_of_stdout("empty-error", r#"{"ok":false,"error":""}"#)
        .await
        .expect_err("ok: false is a failure");
    assert!(
        matches!(&error, OmniFocusError::OmniFocus(message) if message == "OmniFocus script error."),
        "{error:?}"
    );
}

#[tokio::test]
async fn failed_envelope_with_blank_error_reads_as_a_script_error() {
    let error = outcome_of_stdout("blank-error", r#"{"ok":false,"error":"   "}"#)
        .await
        .expect_err("ok: false is a failure");
    assert!(
        matches!(&error, OmniFocusError::OmniFocus(message) if message == "OmniFocus script error."),
        "{error:?}"
    );
}

#[tokio::test]
async fn failed_envelope_passes_its_message_through() {
    let error = outcome_of_stdout(
        "with-error",
        r#"{"ok":false,"error":" Task not found: x "}"#,
    )
    .await
    .expect_err("ok: false is a failure");
    assert!(
        matches!(&error, OmniFocusError::OmniFocus(message) if message == "Task not found: x"),
        "{error:?}"
    );
}

#[tokio::test]
async fn successful_envelope_without_data_is_null() {
    let value = outcome_of_stdout("no-data", r#"{"ok":true}"#)
        .await
        .expect("ok: true is a success");
    assert_eq!(value, Value::Null);
}

#[tokio::test]
async fn successful_envelope_returns_its_data() {
    let value = outcome_of_stdout("with-data", r#"{"ok":true,"data":{"count":3}}"#)
        .await
        .expect("ok: true is a success");
    assert_eq!(value, json!({"count": 3}));
}

// ------------------------------------------------ which osascript (audit CR-040)

/// osascript is named by its absolute path. Looked up through PATH, a
/// directory the user can write to and that precedes /usr/bin would decide
/// which program receives every script.
#[test]
fn production_process_runs_the_system_osascript_by_absolute_path() {
    let process = JxaProcess::osascript();
    assert_eq!(process.program(), Path::new("/usr/bin/osascript"));
    assert!(process.program().is_absolute());
}

#[test]
fn the_default_runner_uses_the_production_process() {
    assert_eq!(RealJxaRunner::new().process(), &JxaProcess::osascript());
    assert_eq!(RealJxaRunner::default().process(), &JxaProcess::osascript());
}

// ------------------------------------------------ the script travels on stdin (audit CR-041)

/// Records the arguments (one per line) and standard input it was given,
/// then answers with a successful envelope.
const RECORDING_STUB: &str = "printf '%s\\n' \"$@\" > \"$STUB_DIR/argv\"\n\
     cat > \"$STUB_DIR/stdin\"\n\
     printf '{\"ok\":true,\"data\":1}'";

#[tokio::test]
async fn the_script_reaches_the_program_on_stdin_and_not_in_its_arguments() {
    let stub = stub("stdin-not-argv", RECORDING_STUB);
    let script = "return 'cr041-marker';";

    let value = run_script(&stub, script).await.expect("the stub succeeds");
    assert_eq!(value, json!(1));

    let argv = fs::read_to_string(stub.dir.join("argv")).expect("the stub recorded argv");
    let argv: Vec<&str> = argv.lines().collect();
    assert_eq!(argv, ["-l", "JavaScript", "-"]);

    let stdin = fs::read_to_string(stub.dir.join("stdin")).expect("the stub recorded stdin");
    assert_eq!(stdin, omnijs_jxa_script(script));
    assert!(stdin.contains("cr041-marker"), "{stdin}");
}

/// A script over the system's argument limit (ARG_MAX, 1 MiB on macOS)
/// could not be started at all as a command-line argument.
#[tokio::test]
async fn a_script_larger_than_the_argument_limit_still_runs() {
    let stub = stub("over-arg-max", RECORDING_STUB);
    let script = format!("// {}\nreturn 1;", "x".repeat(2 * 1024 * 1024));

    let value = run_script(&stub, &script)
        .await
        .expect("a 2 MiB script runs");
    assert_eq!(value, json!(1));

    let received = fs::metadata(stub.dir.join("stdin")).expect("the stub recorded stdin");
    assert_eq!(received.len(), omnijs_jxa_script(&script).len() as u64);
}

/// A program that writes its output before it has read all of its input
/// must not wedge the call: the script is fed while the output is read.
#[tokio::test]
async fn output_written_before_the_script_is_read_does_not_block_the_call() {
    let stub = stub(
        "output-before-input",
        "head -c 262144 /dev/zero | tr '\\0' ' '\n\
         printf '{\"ok\":true,\"data\":2}'\n\
         cat > /dev/null",
    );
    let script = format!("// {}\nreturn 2;", "y".repeat(262_144));

    let value = runner(&stub, Duration::from_secs(5))
        .run_omnijs(&script)
        .await
        .expect("the call completes");
    assert_eq!(value, json!(2));
}

// ------------------------------------------------ unpaired surrogates (audit CR-042)

#[tokio::test]
async fn a_result_holding_an_unpaired_surrogate_still_reads() {
    let value = outcome_of_stdout(
        "lone-surrogate",
        r#"{"ok":true,"data":[{"name":"Plan \ud83d"},{"name":"\ud83d\ude00"}]}"#,
    )
    .await
    .expect("one damaged name must not fail the read");
    assert_eq!(value, json!([{"name": "Plan \u{FFFD}"}, {"name": "😀"}]));
}

/// A script bigger than a pipe's buffer, so a program that never reads it
/// makes the write fail.
fn script_larger_than_a_pipe() -> String {
    format!("// {}\nreturn 1;", "z".repeat(1024 * 1024))
}

#[tokio::test]
async fn a_program_that_succeeds_without_reading_the_script_is_an_io_error() {
    let stub = stub("exit-0-unread", "printf '{\"ok\":true,\"data\":1}'\nexit 0");
    let error = run_script(&stub, &script_larger_than_a_pipe())
        .await
        .expect_err("an answer to a script never read is not the script's answer");
    assert!(
        matches!(&error, OmniFocusError::Io(io) if io.kind() == std::io::ErrorKind::BrokenPipe),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_program_that_fails_without_reading_the_script_reports_its_stderr() {
    let stub = stub("exit-1-unread", "echo 'refused' >&2\nexit 1");
    let error = run_script(&stub, &script_larger_than_a_pipe())
        .await
        .expect_err("exit 1 is a failure");
    assert!(
        matches!(&error, OmniFocusError::JxaExecution(message) if message == "refused"),
        "{error:?}"
    );
}

async fn run_script(stub: &Stub, script: &str) -> omnifocus_mcp::error::Result<Value> {
    runner(stub, GENEROUS_LIMIT).run_omnijs(script).await
}
