# Contributing to OmniFocus MCP

Thanks for contributing. Keep changes small, focused, and backed by tests.

## Development setup

You need macOS and a Rust toolchain from [rustup](https://rustup.rs).

```bash
git clone https://github.com/ChrisGVE/OmnifocusMCP.git
cd OmnifocusMCP/rust
cargo build
```

[`rust/README.md`](rust/README.md) maps the source tree. Installing and configuring a client:
[`docs/install-rust.md`](docs/install-rust.md).

## Checks

A pull request must pass the same checks CI runs (on `macos-latest`):

```bash
cd rust && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

`cargo test` never contacts OmniFocus. Most tests use a mocked script runner. Some run the
Omni Automation snippets in macOS's own JavaScriptCore shell (`jsc`, part of the system), against
hand-built fake objects; they fail rather than skip when `jsc` is missing, which is why the test
suite needs macOS.

## Tests against a live OmniFocus database

The integration tests and the smoke test **create and delete real items in your OmniFocus
database**. Run them only against a database you can afford to modify, with OmniFocus running and
Automation permission granted to your terminal.

| Command | What it runs |
| --- | --- |
| `OMNIFOCUS_INTEGRATION=1 cargo test --features integration` | The integration tests in `tests/integration_test.rs`. |
| `OMNIFOCUS_INTEGRATION=1 OMNIFOCUS_PLAN_C_INTEGRATION=1 cargo test --features integration` | The same, plus `test_plan_c_alias_inputs_work_for_task_tools`, which checks parameter aliases on task tools. |
| `OMNIFOCUS_INTEGRATION=1 OMNIFOCUS_SMOKE=1 cargo run --example smoke_test` | `examples/smoke_test.rs`, which calls a broad set of tools end to end. |
| `cargo run --example probe` | `examples/probe.rs`. Read-only: prints the number of tasks. It has no environment gate. |

Without `OMNIFOCUS_INTEGRATION=1`, or when OmniFocus is not running,
`cargo test --features integration` reports every integration test as passed without testing
anything, and the smoke test prints `smoke test skipped` and exits successfully. A green run
without the variable proves nothing about OmniFocus.

## Key rules

- Insert every user-supplied value into a script through `escape_for_jxa` (`rust/src/jxa.rs`).
- Every parameter struct carries `#[serde(deny_unknown_fields)]`. Integer, number and boolean
  fields use `LenientI32`, `LenientF64` and `LenientBool` (`rust/src/lenient_scalars.rs`); tag
  lists use `FlexibleTagList` (`rust/src/flexible_tags.rs`).
- Date parsing, project status, review intervals and folder/project lookup use the shared
  Omni Automation snippets in `rust/src/js_helpers.rs` (`JS_DATE_HELPERS`, `JS_PROJECT_STATUS`,
  `JS_REVIEW_INTERVAL`, `JS_RESOLVERS`). Prepend the snippet; do not write another copy.
- Every bug fix starts with a failing unit test that reproduces it.
- A change to a tool's parameters or behaviour updates [`docs/tools.md`](docs/tools.md) and
  [`CHANGELOG.md`](CHANGELOG.md) in the same pull request.

## Pull request guidelines

- One concern per pull request; small diffs are preferred.
- Include a short rationale and testing notes, and say whether you ran the live tests.
- Formatting, Clippy and tests must pass before review.
