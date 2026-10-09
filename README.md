# OmniFocus MCP

[![Platform: macOS](https://img.shields.io/badge/platform-macOS-black)](https://www.omnigroup.com/omnifocus)
[![Protocol: MCP](https://img.shields.io/badge/protocol-MCP-6f42c1)](https://modelcontextprotocol.io)
[![Language: Rust](https://img.shields.io/badge/language-Rust-0ea5e9)](rust/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

An [MCP](https://modelcontextprotocol.io) server that lets an AI assistant (the model in your MCP
client) read and change the tasks, projects, tags and folders in
[OmniFocus](https://www.omnigroup.com/omnifocus) on macOS. It is a single binary,
`omnifocus-mcp`, that speaks MCP over stdio and exposes 48 tools, 3 resources and 4 prompts.

This project is not affiliated with, endorsed by, or associated with The Omni Group or OmniFocus.
OmniFocus is a trademark of The Omni Group. This is an independent, non-commercial open-source
project.

## About this fork

This is a maintained fork of
[vitalyrodnenko/OmnifocusMCP](https://github.com/vitalyrodnenko/OmnifocusMCP). Upstream shipped
the same server in Python, TypeScript and Rust; this fork keeps only the Rust server and fixes
bugs reported upstream. Version 2.0.0 is the fork's first release. If you run the upstream
server today, read [Upgrade from the upstream server](#upgrade-from-the-upstream-server) before
installing.

## Contents

- [Requirements](#requirements)
- [Install](#install)
- [Configure your MCP client](#configure-your-mcp-client)
- [Upgrade from the upstream server](#upgrade-from-the-upstream-server)
- [What it can do](#what-it-can-do)
- [Parameters and dates](#parameters-and-dates)
- [Deletes and confirmation](#deletes-and-confirmation)
- [Known limitations](#known-limitations)
- [How it works](#how-it-works)
- [Contributing](#contributing)
- [License](#license)

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

Enable only one OmniFocus MCP server per client. This server and the upstream one offer the same
tool names, so with both enabled the assistant could call either.

## Upgrade from the upstream server

The upstream project also publishes a Homebrew formula named `omnifocus-mcp` that installs a
binary named `omnifocus-mcp`. Remove it before installing this one:

```bash
brew uninstall vitalyrodnenko/omnifocus-mcp/omnifocus-mcp
brew untap vitalyrodnenko/omnifocus-mcp
brew install ChrisGVE/tap/omnifocus-mcp
```

The install guide's
[Upgrade from the upstream server](docs/install-rust.md#upgrade-from-the-upstream-server) explains
why, and what to change in a client that ran the upstream Python or TypeScript server. Calls made
with parameter names a tool does not declare, which upstream ignored, now fail with an error. The
breaking changes are listed at the top of the
[2.0.0 changelog entry](CHANGELOG.md#200---2026-10-07).

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
state, and date ranges on due, defer, completion, planned, added (creation) and changed
(last-modified) dates. `list_tasks` and `search_tasks` also filter by status (`available`,
`due_soon`, `overdue`, `on_hold`, `completed`, `all`; default `available`) and sort.
`get_task_counts` returns counts instead of tasks, so the client receives a few numbers rather than
a task list.

Three tools set a project's status, and the statuses mean different things:

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
| Prompt | `daily_review` | Due-soon, overdue and flagged tasks (the flagged list includes completed ones), for a daily plan |
| Prompt | `weekly_review` | Active projects and available tasks, for a weekly review |
| Prompt | `inbox_processing` | Inbox tasks, to clarify one by one |
| Prompt | `project_planning` | One project and its tasks; takes a required `project` argument (id or exact name) |

## Parameters and dates

Parameter names follow two conventions, and an unknown key is an error rather than being ignored:

- Keys in snake_case only: every key that takes an id (`task_id`, `task_ids`, `parent_task_id`,
  `project_id_or_name`, `tag_name_or_id`, and so on), the three keys `object_type`, `rule_string`
  and `schedule_type`, and the four filters `added_before`, `added_after`, `changed_before`,
  `changed_after`.
- Keys in camelCase that also accept the snake_case spelling: every other multi-word key, for
  example `dueDate` or `due_date`, `sortBy` or `sort_by`.
- A key the tool does not declare, such as `taskId`, fails the call with an `invalid_params` error.

Keys differ per tool, so check the full list of snake_case keys in
[`docs/tools.md`](docs/tools.md#parameter-names) before writing a call by hand.

Numbers and booleans may be sent as strings (`"30"`, `"true"`), and `tags` may be a JSON array
or a string holding one.

A bare date such as `2026-10-10` means that day in your local time zone:

- Written to a due date, it gets the default due time set in OmniFocus (17:00 out of the box; the
  server also uses 17:00 if it cannot read the setting).
- Written to a defer date, it gets the default start time set in OmniFocus (00:00 out of the box;
  the server also uses 00:00 if it cannot read the setting).
- Written to a planned date, it gets the default planned time set in OmniFocus (the server uses
  09:00 if it cannot read the setting).
- In a filter such as `dueBefore`, it means local midnight at the start of that day.

A date-time with `Z` or an offset (`2026-10-10T09:30:00+02:00`) is used as given. An invalid date
such as `2026-02-30` fails the call before anything changes.

A `project` or `folder` value may be an id or an exact name; a value that matches nothing is an
error (`Project not found: <value>`). A name that several projects or folders share is refused,
and the error lists their ids so you can pass one instead.

The exact rules, and every tool's parameters, are in [`docs/tools.md`](docs/tools.md#conventions).

## Deletes and confirmation

The delete tools (`delete_task`, `delete_tasks_batch`, `delete_project`, `delete_projects_batch`,
`delete_tag`, `delete_tags_batch`, `delete_folder`, `delete_folders_batch`) act as soon as they
are called. Deleting a project deletes its tasks.

The server does not ask for confirmation itself. The descriptions of seven delete tools, all but
`delete_tag`, tell the assistant to ask for your approval first, and those of `delete_project` and
the four batch deletes also tell it to show you what will be deleted. Whether it does so is up to
the assistant and the client. If you want a guarantee, keep your client's per-tool approval enabled
for these tools. In Claude Code they are named `mcp__omnifocus__delete_task` and so on.

At startup the server also sends MCP `instructions`, which clients that support them pass to the
assistant. They ask it to:

- base answers on OmniFocus data, and show names before raw ids;
- use `complete_project` for finished work, and `dropped` or `on_hold` only for abandoned or
  paused projects;
- edit or move objects instead of deleting and recreating them;
- confirm before any destructive call;
- report the ids of the objects it changed.

## Known limitations

- **A shared name must be given as an id.** A name used by several projects, folders or tags is
  refused (`Ambiguous project name "Errands": 2 matches (…); pass an id.`), and the message lists
  the ids to choose from.
- **Dates cannot be cleared.** `update_task` and `update_project` treat `null` like an omitted
  field.
- **Review intervals can only be changed, not added.** `update_project` sets `reviewInterval` only
  on a project that already has one.
- **Each call has a 30-second limit.** Calls run one at a time, and a very large `list_tasks` with
  `status: "all"` may hit the limit.
- **Resources are capped at 100 items.** `omnifocus://today` returns up to 100 per section.
- **Planned dates need an OmniFocus version that has them.** On an older version,
  every returned `plannedDate` is `null`. Requests using `plannedBefore`, `plannedAfter`, or a
  planned-date sort fail explicitly because the database cannot honour them, and writing a
  `plannedDate` fails with `plannedDate requires an OmniFocus database migrated to support planned
  dates`.
- **The `daily_review` flagged list can be filled by completed tasks.** It takes the first 25
  flagged tasks in OmniFocus order, completed or not, so in a database with many completed flagged
  tasks the current ones can be left out.
- **Some areas are not covered.** No tool handles attachments, marks a project as reviewed, drops
  a single task, or reads the contents of a perspective.

## How it works

Each tool call runs `/usr/bin/osascript -l JavaScript` (by absolute path, so no other `osascript`
earlier on `PATH` can stand in for it) and writes a short JXA (JavaScript for Automation) script to
its standard input.
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
