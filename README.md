# OmniFocus MCP

[![Platform: macOS](https://img.shields.io/badge/platform-macOS-black)](https://www.omnigroup.com/omnifocus)
[![Protocol: MCP](https://img.shields.io/badge/protocol-MCP-6f42c1)](https://modelcontextprotocol.io)
[![Language: Rust](https://img.shields.io/badge/language-Rust-0ea5e9)](rust/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

An [MCP](https://modelcontextprotocol.io) server that lets an AI assistant read and change the
tasks, projects, tags and folders in [OmniFocus](https://www.omnigroup.com/omnifocus) on macOS. It
is a single binary, `omnifocus-mcp`, that speaks MCP over stdio and exposes 48 tools, 3 resources
and 4 prompts.

This project is not affiliated with, endorsed by, or associated with The Omni Group or OmniFocus.
OmniFocus is a trademark of The Omni Group. This is an independent, non-commercial open-source
project.

## About this fork

This is a maintained fork of
[vitalyrodnenko/OmnifocusMCP](https://github.com/vitalyrodnenko/OmnifocusMCP). Upstream shipped
the same server in Python, TypeScript and Rust; this fork keeps only the Rust server and fixes
bugs reported upstream. Version 2.0.0 is the fork's first release. If you run the upstream
server today, read [Upgrading from upstream](#upgrading-from-upstream) before installing.

## Contents

- [Requirements](#requirements)
- [Install](#install)
- [Configure your MCP client](#configure-your-mcp-client)
- [Upgrading from upstream](#upgrading-from-upstream)
- [What it can do](#what-it-can-do)
- [Parameters and dates](#parameters-and-dates)
- [Deletes and confirmation](#deletes-and-confirmation)
- [Known limitations](#known-limitations)
- [How it works](#how-it-works)
- [Contributing](#contributing)

Full tool reference: [`docs/tools.md`](docs/tools.md). Installation details and troubleshooting:
[`docs/install-rust.md`](docs/install-rust.md).

## Requirements

- macOS. OmniFocus is a macOS app and the server drives it through `osascript`.
- OmniFocus installed and running whenever a tool is called. Tested with OmniFocus 4.9.2 Pro.
- macOS Automation permission for the app that starts the server (your terminal for Claude Code,
  or Claude Desktop, Cursor, and so on) to control OmniFocus. macOS asks on the first tool call;
  you can change it later in System Settings > Privacy & Security > Automation.
- A Rust toolchain only if you build from source.

## Install

Install the prebuilt binary (Apple silicon or Intel) from the `ChrisGVE/tap` Homebrew tap:

```bash
brew install ChrisGVE/tap/omnifocus-mcp
omnifocus-mcp --version
```

The second command prints `omnifocus-mcp 2.0.0`. To install from a release tarball or build from
source, see [`docs/install-rust.md`](docs/install-rust.md).

## Configure your MCP client

Claude Code, which inherits your shell's `PATH`:

```bash
claude mcp add --scope user omnifocus -- omnifocus-mcp
```

Claude Desktop, Cursor and other stdio clients take a JSON entry. Apps opened from the Dock or
Finder do not see your shell's `PATH`, so give the absolute path. Print it with
`echo "$(brew --prefix)/bin/omnifocus-mcp"`; it is usually `/opt/homebrew/bin/omnifocus-mcp` on
Apple silicon and `/usr/local/bin/omnifocus-mcp` on Intel.

```json
{
  "mcpServers": {
    "omnifocus": {
      "command": "/opt/homebrew/bin/omnifocus-mcp",
      "args": []
    }
  }
}
```

Enable only one OmniFocus MCP server in a client. This server and the upstream one offer the same
tool names, so with both enabled the assistant could call either.

## Upgrading from upstream

The upstream Homebrew formula has the same name and installs a binary with the same name,
`omnifocus-mcp`. Remove it first, so that the binary on your `PATH` is this one:

```bash
brew uninstall vitalyrodnenko/omnifocus-mcp/omnifocus-mcp
brew untap vitalyrodnenko/omnifocus-mcp
brew install ChrisGVE/tap/omnifocus-mcp
```

If your client started the upstream Python or TypeScript server, replace that entry's `command`
and `args` with the binary as shown above.

A client entry that already runs `omnifocus-mcp` needs no change. Calls made with the wrong
parameter names, which upstream ignored, now fail with an error; the breaking changes are listed
at the top of the [2.0.0 changelog entry](CHANGELOG.md#200---2026-10-07).

## What it can do

Each tool is described, with every parameter, in [`docs/tools.md`](docs/tools.md).

| Area | Tools |
| --- | --- |
| Tasks: read (6) | `get_inbox`, `list_tasks`, `search_tasks`, `get_task`, `list_subtasks`, `get_task_counts` |
| Tasks: create and edit (6) | `create_task`, `create_tasks_batch`, `create_subtask`, `update_task`, `duplicate_task`, `append_to_note` (tasks and projects) |
| Tasks: complete (2) | `complete_task`, `uncomplete_task` |
| Tasks: move (2) | `move_task`, `move_tasks_batch` |
| Tasks: repetition and notifications (4) | `set_task_repetition`, `list_notifications`, `add_notification`, `remove_notification` |
| Tasks: delete (2) | `delete_task`, `delete_tasks_batch` |
| Projects: read (4) | `list_projects`, `search_projects`, `get_project`, `get_project_counts` |
| Projects: create and edit (3) | `create_project`, `update_project`, `move_project` |
| Projects: status (3) | `complete_project`, `uncomplete_project`, `set_project_status` |
| Projects: delete (2) | `delete_project`, `delete_projects_batch` |
| Tags (6) | `list_tags`, `search_tags`, `create_tag`, `update_tag`, `delete_tag`, `delete_tags_batch` |
| Folders (6) | `list_folders`, `get_folder`, `create_folder`, `update_folder`, `delete_folder`, `delete_folders_batch` |
| Forecast (1) | `get_forecast`: overdue, due today, flagged, deferred, due within 7 days |
| Perspectives (1) | `list_perspectives`: names and ids only |

That is 22 task tools, 12 project tools, 6 tag tools, 6 folder tools and one each for the forecast
and perspectives.

`list_tasks`, `search_tasks` and `get_task_counts` filter by project, tags (any or all), flagged
state, and date ranges on due, defer, completion, planned, creation and last-modified dates.
`list_tasks` and `search_tasks` also filter by status (`available`, `due_soon`, `overdue`,
`on_hold`, `completed`, `all`; default `available`) and sort. `get_task_counts` returns counts
instead of tasks, so the client receives a few numbers rather than a task list.

Project status has three tools, which mean different things:

| Status | Meaning | Set with |
| --- | --- | --- |
| `completed` | The work is finished. | `complete_project` (`uncomplete_project` reopens it) |
| `dropped` | The project was abandoned, not finished. | `set_project_status` |
| `on_hold` | Paused. | `set_project_status` |
| `active` | Current. | `set_project_status` |

Resources and prompts:

| Kind | Identifier | Content |
| --- | --- | --- |
| Resource | `omnifocus://inbox` | Inbox tasks (up to 100) |
| Resource | `omnifocus://today` | The `get_forecast` sections (up to 100 tasks each) |
| Resource | `omnifocus://projects` | Active projects with task counts (up to 100) |
| Prompt | `daily_review` | Due-soon, overdue and flagged tasks, for a daily plan |
| Prompt | `weekly_review` | Active projects and available tasks, for a weekly review |
| Prompt | `inbox_processing` | Inbox tasks, to clarify one by one |
| Prompt | `project_planning` | One project and its tasks; takes a required `project` argument (id or exact name) |

## Parameters and dates

Parameter names follow two conventions, and an unknown key is an error rather than being ignored:

- Identifiers are snake_case only: `task_id`, `parent_task_id`, `project_id_or_name`, and so on.
  So are four task filters: `added_before`, `added_after`, `changed_before`, `changed_after`.
- Every other multi-word key is camelCase and also accepts the snake_case spelling: `dueDate` or
  `due_date`, `sortBy` or `sort_by`.
- A key the tool does not declare, such as `taskId`, fails the call with an `invalid_params` error.

Numbers and booleans may be sent as strings (`"30"`, `"true"`), and `tags` may be a JSON array
or a string holding one.

A bare date such as `2026-10-10` means that day in your local time zone:

- Written to a due date, it gets the default due time set in OmniFocus (17:00 out of the box).
- Written to a defer date, it gets the default start time set in OmniFocus (00:00 out of the box).
- In a filter such as `dueBefore`, it means local midnight at the start of that day.

A date-time with `Z` or an offset (`2026-10-10T09:30:00+02:00`) is used as given. An invalid date
such as `2026-02-30` fails the call before anything changes.

A `project` or `folder` value may be an id or an exact name; a value that matches nothing is an
error (`Project not found: <value>`).

The exact rules, and every tool's parameters, are in [`docs/tools.md`](docs/tools.md#conventions).

## Deletes and confirmation

The delete tools (`delete_task`, `delete_tasks_batch`, `delete_project`, `delete_projects_batch`,
`delete_tag`, `delete_tags_batch`, `delete_folder`, `delete_folders_batch`) act as soon as they
are called. Deleting a project deletes its tasks.

The server does not ask for confirmation itself. Each delete tool's description tells the model to
show you what will be deleted and wait for your approval, but whether the model does so is up to
the model and the client. If you want a guarantee, keep your client's per-tool approval enabled for
these tools. In Claude Code they are named `mcp__omnifocus__delete_task` and so on.

At startup the server also sends MCP `instructions`, which clients that support them pass to the
model. They ask the model to base answers on OmniFocus data, show names before raw ids, use
`complete_project` for finished work and `dropped` or `on_hold` only for abandoned or paused
projects, edit or move objects instead of deleting and recreating them, confirm before any
destructive call, and report the ids of objects it changed.

## Known limitations

- **Task status filters ignore dropped tasks and project status.** `available`, `overdue` and
  `due_soon` (in `list_tasks` and `search_tasks`), the counts of `get_task_counts`, `get_forecast`,
  and the review prompts check only whether the task itself is marked completed. They therefore
  include dropped tasks, and tasks in a completed or dropped project unless the task itself is
  marked completed. Each returned task carries OmniFocus's own `taskStatus`, which does report
  `dropped`.
- **`available` means two things.** In `list_tasks` and `search_tasks` it is every task not
  completed, deferred ones included. In `get_task_counts` it excludes tasks deferred to the future.
- **Duplicate names resolve to the first match.** A name shared by several projects, folders or
  tags selects the first one OmniFocus lists. Use the id to pick a specific one.
- **Tags are matched by name.** On `create_task`, `update_task` and `update_project`, a tag name
  that matches no existing tag is skipped without an error; tags are not created implicitly.
- **Dates cannot be cleared.** `update_task` and `update_project` treat `null` like an omitted
  field.
- **Review intervals can only be changed, not added.** `update_project` sets `reviewInterval` only
  on a project that already has one.
- **Each call has a 30-second limit**, and calls run one at a time. A very large `list_tasks` with
  `status: "all"` may hit the limit.
- **Resources are capped** at 100 items (100 per section for `omnifocus://today`).
- **Planned-date filters are ignored** on an OmniFocus version without planned dates.
- **Not covered:** attachments, marking a project as reviewed, dropping a single task, and reading
  the contents of a perspective.

## How it works

Each tool call runs `osascript -l JavaScript` with a short JXA (JavaScript for Automation) script.
That script hands a second script to OmniFocus's `evaluateJavascript`, which runs it inside
OmniFocus in Omni Automation, OmniFocus's built-in JavaScript API (also called OmniJS, the name the
tool descriptions use). The two steps are needed because objects such as `flattenedTasks` and
`Task.Status` exist only in Omni Automation. The script returns JSON, which the server passes back
to the client.

## Contributing

Pull requests are welcome. [`CONTRIBUTING.md`](CONTRIBUTING.md) covers the build, the checks a pull
request must pass, and how to run the tests that act on a live OmniFocus database.

## License

MIT. See [`LICENSE`](LICENSE).
