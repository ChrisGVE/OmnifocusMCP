# Changelog

All notable changes to this project are documented in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Version 2.0.0 is the first release of this fork. The entries for 1.1.9 and earlier are the history
of the upstream project, [vitalyrodnenko/OmnifocusMCP](https://github.com/vitalyrodnenko/OmnifocusMCP);
issue numbers in them, and every "upstream #N" below, refer to that project's tracker.

## [Unreleased]

## [2.0.0] - 2026-10-07

This is a major release because existing calls can now fail or behave differently. Those changes
are marked **Breaking**. Upgrade steps:
[Upgrade from the upstream server](docs/install-rust.md#upgrade-from-the-upstream-server).

### Removed
- **Breaking:** the Python and TypeScript implementations, their install guides and CI jobs, the
  root `package.json`, and the Homebrew formula template. Only the Rust server (`omnifocus-mcp`)
  ships. Its Homebrew formula is in the `ChrisGVE/tap` tap:
  `brew install ChrisGVE/tap/omnifocus-mcp`.

### Added
- `folder` and `project` parameters accept an id as well as an exact name: on `list_projects`,
  `get_project_counts`, `create_project`, `move_project`, `create_folder` (`parent`), `list_tasks`,
  `search_tasks`, `get_task_counts`, `create_task`, `create_tasks_batch`, `move_task` and
  `move_tasks_batch`. Filters then compare by id, so two projects with the same name are told
  apart. ([upstream #11](https://github.com/vitalyrodnenko/OmnifocusMCP/issues/11))
- A reference of every tool and parameter: [`docs/tools.md`](docs/tools.md).

### Changed
- **Breaking:** tool and prompt parameters reject unknown keys instead of ignoring them. A call
  with a misspelled or undeclared key (for example `folderId` where the tool declares `folder`)
  fails with an `invalid_params` error instead of succeeding without its argument. Every tool
  schema now advertises `additionalProperties: false`.
- **Breaking:** a bare `YYYY-MM-DD` date is read as a day in your local time zone, not as UTC
  midnight. Written to `dueDate` or `deferDate` (tasks and projects), it gets the default due or
  start time set in OmniFocus (17:00 and 00:00 if the setting cannot be read), as the OmniFocus
  app does. In filters (`dueBefore`, `completedAfter`, ...) and in `add_notification`'s
  `absoluteDate` it is local midnight. Date-times with `Z` or an offset are unchanged.
- **Breaking:** an invalid date (for example `2026-02-30`) fails with an error naming the field,
  before anything is created or changed. Some write paths used to store an invalid date silently.
- **Breaking:** a `folder` or `project` value (including `create_folder`'s `parent`) that matches
  nothing fails with `Folder not found: <value>` or `Project not found: <value>`, instead of
  returning an empty result or placing the new object at the top level.
  ([upstream #11](https://github.com/vitalyrodnenko/OmnifocusMCP/issues/11))
- **Breaking:** a project status the server does not recognise is reported as `"unknown"` instead
  of `"active"`.
- The `project_planning` prompt lists only the tasks of the project it resolved, by id. A project
  that does not exist still produces a prompt, with status `not_found` and no tasks.

### Fixed
- Integer, number and boolean parameters accept their string encoding (`"30"`, `"true"`), as sent
  by MCP clients that serialize every argument as a string. Integral floats (`30.0`) are accepted
  for integers. The advertised schema is unchanged.
  ([upstream #8](https://github.com/vitalyrodnenko/OmnifocusMCP/issues/8),
  [#11](https://github.com/vitalyrodnenko/OmnifocusMCP/issues/11))
- Bare dates were parsed as UTC midnight, so outside UTC tasks were deferred hours late or to the
  wrong day. ([upstream #13](https://github.com/vitalyrodnenko/OmnifocusMCP/issues/13))
- Completed projects were reported, filtered and counted as `active`, because OmniFocus's
  `Project.Status.Done` was not recognised. Also fixed in `get_folder`'s project list.
  ([upstream #10](https://github.com/vitalyrodnenko/OmnifocusMCP/issues/10); diagnosis from
  [upstream PR #14](https://github.com/vitalyrodnenko/OmnifocusMCP/pull/14) by @luebbers)
- `update_project` could never set `reviewInterval`: it assigned a plain object and singularised
  the unit. It now accepts `"N unit"` (unit `day`, `week`, `month` or `year`, singular or plural),
  validates it before anything changes, and fails if the project has no review interval to change.
  Read tools report the interval as `"2 weeks"` or `"1 week"` instead of
  `"[object Project.ReviewInterval]"`.
  ([upstream #12](https://github.com/vitalyrodnenko/OmnifocusMCP/issues/12))
- A project's folder was read from `project.folder`, which OmniFocus does not provide, so
  `folderName`, the folder filters of `list_projects` and `get_project_counts`, and the
  `projectCount` of `list_folders` came back empty. They now read `parentFolder`.
  ([upstream #11](https://github.com/vitalyrodnenko/OmnifocusMCP/issues/11))
- `move_project` reports the folder the project ended up in, not the one requested.
- `create_tasks_batch` resolves every destination project and parses every date before creating
  any task, so one bad entry no longer leaves part of the batch created.
- `get_task_counts` accepted `plannedBefore` and `plannedAfter` but ignored them; it now applies
  them.
- The server identifies itself to MCP clients as `omnifocus-mcp` with its version, instead of the
  name and version of the MCP library it is built on (`rmcp 0.17.0`).
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

[Unreleased]: https://github.com/ChrisGVE/OmnifocusMCP/compare/rust-v2.0.0...HEAD
[2.0.0]: https://github.com/ChrisGVE/OmnifocusMCP/compare/rust-v1.1.9...rust-v2.0.0
[1.1.9]: https://github.com/vitalyrodnenko/OmnifocusMCP/compare/rust-v1.1.8...rust-v1.1.9
[1.1.8]: https://github.com/vitalyrodnenko/OmnifocusMCP/compare/rust-v1.1.7...rust-v1.1.8
[1.1.7]: https://github.com/vitalyrodnenko/OmnifocusMCP/compare/rust-v1.1.6...rust-v1.1.7
[1.1.6]: https://github.com/vitalyrodnenko/OmnifocusMCP/compare/rust-v1.1.5...rust-v1.1.6
[1.1.5]: https://github.com/vitalyrodnenko/OmnifocusMCP/compare/rust-v1.1.4...rust-v1.1.5
[1.1.4]: https://github.com/vitalyrodnenko/OmnifocusMCP/compare/rust-v1.1.3...rust-v1.1.4
