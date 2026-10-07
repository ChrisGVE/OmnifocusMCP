# OmniFocus MCP

[![Platform: macOS](https://img.shields.io/badge/platform-macOS-black)](https://www.omnigroup.com/omnifocus)
[![Protocol: MCP](https://img.shields.io/badge/protocol-MCP-6f42c1)](https://modelcontextprotocol.io)
[![Language: Rust](https://img.shields.io/badge/impl-rust-0ea5e9)](rust/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

MCP server that gives AI assistants full control over [OmniFocus](https://www.omnigroup.com/omnifocus) on macOS.

48 tools, 3 resources, and 4 prompts covering tasks, projects, tags, folders, perspectives, forecast, notifications, and review workflows — all through the [Model Context Protocol](https://modelcontextprotocol.io).

This project is not affiliated with, endorsed by, or associated with The Omni Group or OmniFocus. OmniFocus is a trademark of The Omni Group. This is an independent, non-commercial open-source project.

## About this fork

This is a maintained fork of [vitalyrodnenko/OmnifocusMCP](https://github.com/vitalyrodnenko/OmnifocusMCP),
reduced to the Rust server. Upstream shipped the same server in Python, TypeScript and Rust; this
fork keeps only Rust and fixes the bugs reported upstream. The upstream Homebrew tap
(`vitalyrodnenko/omnifocus-mcp`) installs the upstream build, not this one.

## Quick Start

Build from source (Rust toolchain via [`rustup`](https://rustup.rs)):

```bash
git clone https://github.com/ChrisGVE/OmnifocusMCP.git
cd OmnifocusMCP/rust
cargo build --release
```

Then add the binary to your MCP client config (Claude Desktop, Cursor, etc.):

```json
{
  "mcpServers": {
    "omnifocus": {
      "command": "/absolute/path/to/OmnifocusMCP/rust/target/release/omnifocus-mcp",
      "args": []
    }
  }
}
```

The AI assistant now has full OmniFocus access.

## What It Can Do

### Tasks (22 tools)

Full lifecycle management for OmniFocus tasks:

- **CRUD** — create, get, update, delete individual tasks
- **Batch operations** — create, move, or delete multiple tasks in a single call
- **Subtasks** — create and list subtasks under any parent task
- **Completion** — mark complete, mark incomplete (supports repeating tasks)
- **Search** — full-text search across task names and notes with all filters applied
- **Move and reparent** — relocate tasks between projects, reparent tasks under other tasks, or move subtasks back to inbox/project without delete/recreate
- **Duplicate** — clone a task with all properties and optional subtasks
- **Notifications** — list, add, and remove notifications (absolute date or relative offset)
- **Repetition** — set or clear repetition rules with schedule type (regularly/after completion)
- **Notes** — append text to task notes without overwriting
- **Safety model** — destructive delete confirmations stay separate from non-destructive move/update workflows
- **Aggregate counts** — fast "how many" queries without listing individual tasks

#### Advanced Filtering

`list_tasks` and `search_tasks` support powerful filter combinations:

| Filter | Description |
| --- | --- |
| `project` | Scope to a single project, by id or exact name (unknown project → error) |
| `tag` / `tags` | Filter by one tag or multiple tags |
| `tagFilterMode` | `"any"` (default) or `"all"` for multi-tag filtering |
| `flagged` | Flagged tasks only |
| `status` | `"available"`, `"remaining"`, `"completed"`, `"dropped"`, `"all"` |
| `dueBefore` / `dueAfter` | Due date range (ISO 8601) |
| `deferBefore` / `deferAfter` | Defer date range (ISO 8601) |
| `completedBefore` / `completedAfter` | Completion date range (ISO 8601) |
| `addedBefore` / `addedAfter` | Creation date range (ISO 8601) |
| `changedBefore` / `changedAfter` | Last-modified date range (ISO 8601, maps to OmniFocus `modified`) |
| `plannedBefore` / `plannedAfter` | Planned date range (ISO 8601) |
| `maxEstimatedMinutes` | Tasks with estimated duration up to N minutes |

#### Sorting

All list/search tools support `sortBy` and `sortOrder`:

- Sort by: `name`, `dueDate`, `deferDate`, `completionDate`, `estimatedMinutes`, `project`, `flagged`, `addedDate`, `changedDate`, `plannedDate`
- Aliases: `added` -> `addedDate`, `modified` -> `changedDate`, `planned` -> `plannedDate`
- Sort order: `asc` (default) or `desc`
- Task payloads include `addedDate` and `changedDate` (ISO 8601 or `null`)

### Projects (12 tools)

- **CRUD** — create, get, update, delete projects
- **Lifecycle** — complete, uncomplete, set status (active/on-hold/dropped)
- **Organization** — move between folders (folder by id or exact name), search by name
- **Filtering** — by folder (id or exact name), status, completion date range, stalled-only flag
- **Review interval** — set with `update_project` as `"N unit"` (`days`, `weeks`, `months`, `years`), reported back the same way
- **Sorting** — by name, due date, or other fields
- **Aggregate counts** — project counts by status, optionally scoped to a folder

#### Project Lifecycle Semantics

- Use `complete_project` when work is finished/closed (done/completed).
- Use `set_project_status` for organizational state only:
  - `active` = current
  - `on_hold` = paused (UI wording is often "on hold"/"on-hold")
  - `dropped` = intentionally abandoned/cancelled, not completed
- Use `uncomplete_project` to reopen a completed project back to active.
- In user-facing summaries, present business meaning first (project name,
  folder, and status transition), and include opaque IDs only as secondary
  references.

### Tags (6 tools)

- **CRUD** — create, update (name and status), delete
- **List** — with status filter (active/on-hold/dropped/all), sorting, and limits
- **Search** — fuzzy name matching

### Folders (6 tools)

- **CRUD** — create, get (with child projects and subfolders), update, delete
- **Hierarchy** — create nested folders with parent parameter
- **List** — all folders with limits

### Forecast (1 tool)

- Structured view with sections: overdue, due today, flagged, deferred, and due this week

### Perspectives (1 tool)

- List all available OmniFocus perspectives

### Resources (3)

Live snapshots available to MCP clients:

| Resource | Description |
| --- | --- |
| Inbox | Current inbox tasks |
| Today | Today's forecast (overdue + due today + flagged) |
| Active Projects | All active projects with task counts |

### Prompts (4)

Ready-to-use review workflows:

| Prompt | Description |
| --- | --- |
| Daily Review | Due-soon, overdue, and flagged tasks for daily planning |
| Weekly Review | Active projects and next-action coverage analysis |
| Inbox Processing | One-by-one inbox clarification decisions |
| Project Planning | Guided planning for a specific project |

## How It Works

The server runs JXA (JavaScript for Automation) scripts through macOS `osascript`. Each script uses the OmniFocus `evaluateJavascript` bridge to execute Omni Automation JavaScript inside OmniFocus itself, where full APIs like `flattenedTasks`, `Task.Status`, and `new Task()` are available. Data is serialized as JSON and returned through the MCP protocol.

## MCP Client Config

Every stdio MCP client uses the same shape; point `command` at the built binary (or just
`omnifocus-mcp` if you copied it onto your `PATH`). Full guide: [`docs/install-rust.md`](docs/install-rust.md).

> Keep only one OmniFocus MCP server enabled at a time to avoid duplicate tool surfaces.

## Prerequisites

- macOS (required — OmniFocus is macOS-only)
- OmniFocus installed and running
- Automation permission granted to the terminal/editor (System Settings → Privacy & Security → Automation)

- Rust toolchain via [`rustup`](https://rustup.rs) to build

## Contributing

Contributions are welcome through focused pull requests with clear scope and passing checks. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for setup and validation steps.

## License

MIT. See [`LICENSE`](LICENSE) for details.
