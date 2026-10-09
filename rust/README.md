# rust/

Source of the `omnifocus-mcp` binary, the Rust MCP server for OmniFocus. This page is for
contributors. To install and use the server, start from the [README](../README.md); every tool is
described in the [tool reference](../docs/tools.md).

## Build and test

```bash
cargo build --release
./target/release/omnifocus-mcp --version

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

`cargo test` needs macOS but never contacts OmniFocus. The integration tests and
`examples/smoke_test.rs` **create and delete real items in your OmniFocus database** and run only
when environment variables enable them; without them they report success without testing
anything. See
[Tests against a live OmniFocus database](../CONTRIBUTING.md#tests-against-a-live-omnifocus-database).

## How a call flows

`main.rs` runs the MCP server defined in `server.rs` on stdio. Each tool handler in `server.rs`
deserializes its parameter struct and calls a function in `src/tools/`. That function validates the
input in Rust, builds an Omni Automation script (prepending shared snippets from `js_helpers.rs`),
and runs it through `jxa.rs`, which wraps it in a JXA script, feeds that to `osascript` on standard
input and turns the JSON reply or error into a result.

## Layout

```
rust/
  Cargo.toml
  src/
    main.rs             # entry point: --version/--help, stdio transport, exit status
    lib.rs              # declares the modules below
    shutdown.rs         # which ends of the server are clean (exit 0) and which are failures
                        #   (logged to stderr, non-zero exit)
    params/             # MCP parameter structs, one file per tool area (tasks, projects,
                        #   tags and folders); re-exported from server
    server/             # OmniFocusServer: mod.rs holds the handler, prompts, resources and
                        #   MCP instructions and adds up one tool router per area file
                        #   (task_read_tools, task_write_tools, notification_tools,
                        #   project_tools, tag_tools, folder_tools, view_tools)
    jxa.rs              # script runner: JxaProcess (program + time limit; /usr/bin/osascript
                        #   with a 30 s limit in production), one call at a time, escape_for_jxa,
                        #   error messages
    error.rs            # OmniFocusError, the error type of every tool; a tool reports it as
                        #   an isError result, a prompt or resource as a JSON-RPC error
    types.rs            # result structs (task summaries, counts)
    js_helpers.rs       # shared Omni Automation snippets: dates, planned dates, project and
                        #   folder status, task status, review intervals, id-or-name resolution
    lenient_scalars.rs  # integer/number/boolean parameters that also accept strings
    flexible_tags.rs    # `tags` as an array or a JSON-array string
    review_interval.rs  # parses "N unit" review intervals
    resources.rs        # omnifocus://inbox, omnifocus://today, omnifocus://projects
    prompts.rs          # daily_review, weekly_review, inbox_processing, project_planning
    tools/
      mod.rs            # declares the tool modules; folders_clean.rs is the module `folders`
      tasks/            # task tools, one file per group: list, search, counts, reads,
                        #   create, update, lifecycle, moves, batch, notifications; the
                        #   shared list/search script lives in listing_script.rs and filters.rs
      projects/         # project tools: list, reads, writes, lifecycle
      utility.rs        # uncomplete_task and append_to_note
                        #   (the server and examples/smoke_test.rs both use these)
      tags.rs           # tag tools
      folders_clean.rs  # folder tools
      forecast.rs       # get_forecast
      perspectives.rs   # list_perspectives
      batch_delete.rs   # id/name validation and summary shared by the batch deletes
      js_values.rs      # Rust values as JavaScript literals
  tests/                # none contacts OmniFocus except where marked LIVE
    common/mod.rs                      # runs snippets in the macOS `jsc` shell
    jxa_test.rs                        # escaping, error messages, reply unwrapping
    jxa_process_test.rs                # the runner's process handling, against a stub program
    jxa_timeout_test.rs                # the time limit as a Duration and how it is named
    jxa_surrogate_test.rs              # unpaired UTF-16 surrogates in OmniFocus output
    params_test.rs                     # wire contract: unknown keys, string-encoded scalars
    lenient_scalars_test.rs            # lenient scalar types
    date_parsing_test.rs               # every date goes through the shared date helpers
    js_date_helpers_test.rs            # date helper behaviour in JavaScriptCore
    folder_project_resolution_test.rs  # id-or-name resolution in JavaScriptCore
    folder_project_schema_test.rs      # advertised folder/project parameter schemas
    folder_project_wiring_test.rs      # which tools use the resolvers
    project_status_test.rs             # project status naming
    review_interval_test.rs            # review interval parsing and assignment
    tools_read_test.rs                 # read tools against a mocked runner
    tools_write_test.rs                # write tools against a mocked runner
    resources_test.rs                  # resource contents
    prompts_test.rs                    # prompt rendering
    server_info_test.rs                # server name and version sent in `initialize`
    task_counts_planned_test.rs        # planned-date filters of get_task_counts
    tool_descriptions_test.rs          # tool descriptions state the date and id-or-name rules
    task_status_helpers_test.rs        # remaining/available/completed/stalled in JavaScriptCore
    task_status_tools_test.rs          # which tools use the task status helpers
    folder_status_test.rs              # folder status naming
    tag_resolution_test.rs             # unknown tag names fail on writes
    planned_date_support_test.rs       # planned-date filters on unmigrated databases
    planned_date_write_test.rs         # plannedDate on the write tools
    integration_test.rs                # LIVE database; needs --features integration and
                                       #   OMNIFOCUS_INTEGRATION=1
  examples/
    probe.rs            # LIVE, read-only: prints the number of tasks
    smoke_test.rs       # LIVE: creates and deletes real items; needs OMNIFOCUS_INTEGRATION=1
                        #   and OMNIFOCUS_SMOKE=1
```

## Releases

[`.github/workflows/release-rust.yml`](../.github/workflows/release-rust.yml) runs on tags named
`rust-v<version>`. It checks that the tag matches the version in `Cargo.toml`, builds
`aarch64-apple-darwin` and `x86_64-apple-darwin` binaries, and publishes them as
`omnifocus-mcp-<version>-<target>.tar.gz` with a `.sha256` file on a GitHub release.
