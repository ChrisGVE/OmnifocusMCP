# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

### Removed
- Python and TypeScript implementations, the Homebrew formula template, their install guides,
  CI jobs, and the root `package.json`. This fork is Rust-only.

### Changed
- Tool parameters now reject unknown keys instead of silently ignoring them. A client that sends
  a misspelled or undeclared key (e.g. `folderId` where the tool declares `folder`) gets an
  `invalid_params` error rather than a success that ignored its argument. Every tool schema now
  advertises `additionalProperties: false`.
- A bare `YYYY-MM-DD` date is read as a local calendar day. On writes (`dueDate`, `deferDate` of
  tasks and projects) it is set at your OmniFocus default time for that field (Settings
  `DefaultDueTime` / `DefaultStartTime`, factory 17:00 / 00:00), as the OmniFocus UI does. In
  filters (`dueBefore`, `completedAfter`, …) and `add_notification`'s `absoluteDate` it is local
  midnight. Date-times with `Z` or an offset are unchanged.
- Invalid dates (e.g. `2026-02-30`) now fail with an error naming the field, before anything is
  created or changed. Previously some write paths silently stored an Invalid Date.
- `folder` and `project` parameters (create/move/list/count tools, task filters and writes)
  accept an id or an exact name. A value that matches nothing is an error (`Folder not found: …`,
  `Project not found: …`) instead of an empty result or a silent top-level placement. Filters
  compare by id, so same-named projects are told apart. (upstream #11)
- Project status unknown to the server is reported as `"unknown"` rather than defaulting to
  `"active"`.
- CI runs `cargo clippy --all-targets`, so test code is linted too.

### Fixed
- Integer, number and boolean parameters accept their string encoding (`"30"`, `"true"`), as sent
  by MCP clients that serialize every argument as a string. Integral floats (`30.0`) are accepted
  for integers. The advertised schema is unchanged. (upstream #8, #11)
- Bare dates were parsed as UTC midnight, deferring tasks hours late (or to the wrong day) outside
  UTC. (upstream #13)
- Completed projects were reported, filtered and counted as `active`: OmniFocus's
  `Project.Status.Done` was not recognised. Also fixed in `get_folder`'s project list.
  (upstream #10; diagnosis from upstream PR #14 by @luebbers)
- `update_project` could never set `reviewInterval` (it assigned a plain object, and singularised
  the unit). The interval is now validated before anything changes and applied to the project's
  own `Project.ReviewInterval`. Read tools report it as `"2 weeks"` instead of
  `"[object Project.ReviewInterval]"`. (upstream #12)
- Project folder was read from an undocumented `project.folder` property, so `folderName`, the
  folder filters and `list_folders`' `projectCount` came back empty; now `parentFolder`.
  `move_project` reports the folder the project actually ended up in, not the requested one.
  `create_tasks_batch` resolves every destination before creating any task. (upstream #11)
- README tool counts: the server registers 48 tools (tasks 22, projects 12, tags 6, folders 6,
  forecast 1, perspectives 1), not 45.

## [1.1.9] - 2026-04-05

### Fixed
- Rust MCP server: `tags` on task and project write tools, and on `list_tasks` / `search_tasks` / `get_task_counts`, now deserialize from either a JSON array of strings or a single string containing a JSON array (e.g. `"[\"Quick\",\"Home\"]"`). This matches MCP clients that serialize all tool parameters as strings.

## [1.1.8] - 2026-04-04

### Fixed
- Rust MCP server: tool input JSON Schema now uses camelCase property names for task write tools (`dueDate`, `deferDate`, `estimatedMinutes`) consistent with Python and TypeScript, so `tags` is accepted as a JSON array and no longer fails MCP parameter deserialization (issue #7).

## [1.1.7] - 2026-03-15

### Fixed
- stabilized live integration behavior for task, project, tag, and folder workflows:
  - hierarchy-safe batch deletion now handles parent+child requests without false partial failures
  - status outputs are normalized to canonical values across tag/folder/project surfaces
  - natural-language aliases are accepted for key filters (`descending`, `due soon`, `on hold`, `AND`/`OR`)

## [1.1.6] - 2026-03-03

### Fixed
- added missing task sort fields in `list_tasks` and `search_tasks` across Python, TypeScript, and Rust:
  - canonical date fields: `addedDate`, `changedDate`, `plannedDate`
  - aliases accepted by clients/LLMs: `added`, `modified`, `planned`
- fixed a TypeScript `create_project` OmniJS script syntax bug that could fail with
  `Unexpected keyword 'catch'` during real integration calls
- updated Rust integration harness calls to match the current task API signature

### Validation
- re-ran real OmniFocus integration suites in Python, TypeScript, and Rust, plus Rust smoke test

## [1.1.5] - 2026-03-05

### Added
- support task date filters for creation/last-modified dates:
  - `added_after`, `added_before`
  - `changed_after`, `changed_before` (`changed` maps to OmniFocus `modified`)
- include `addedDate` and `changedDate` in task payloads returned by read tools

### Changed
- updated documentation in root, Python, TypeScript, and Rust READMEs to describe
  the new date filters and task date fields
- bumped Rust package metadata from `1.1.4` to `1.1.5`

## [1.1.4] - 2026-03-03

### Fixed
- perspective enumeration now includes built-in, custom, and document perspectives
  so completed and custom perspectives are returned correctly

### Added
- batch deletion tools for projects, tags, and folders with partial-success output:
  - `delete_projects_batch`
  - `delete_tags_batch`
  - `delete_folders_batch`
