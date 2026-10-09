# Changelog

All notable changes to this project are documented in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Version 2.0.0 is the first release of this fork. The entries for 1.1.9 and earlier are the history
of the upstream project, [vitalyrodnenko/OmnifocusMCP](https://github.com/vitalyrodnenko/OmnifocusMCP);
issue numbers in them, and every "upstream #N" below, refer to that project's tracker.

## [Unreleased]

### Added
- `create_task`, `create_subtask`, `create_tasks_batch` and `update_task` accept an optional
  `plannedDate`, written the same way as `dueDate` and `deferDate` (a bare `YYYY-MM-DD` becomes
  that local day at the default planned time; the server uses 09:00 when it cannot read the
  setting). Writing a `plannedDate` on a database that has not been migrated for planned dates
  fails with `plannedDate requires an OmniFocus database migrated to support planned dates` before
  anything changes; when the parameter is absent the property is never written. The tasks these
  tools return now include `plannedDate` (ISO string or `null`).
- Every tool declares MCP annotations (`readOnlyHint`, `destructiveHint`, `idempotentHint`,
  `openWorldHint`) in `tools/list`. Without them a client had to treat every tool, `list_tasks`
  included, as destructive. Now the 17 read tools are marked read-only, and only the tools that can
  overwrite or remove existing content are marked destructive: the deletes, `remove_notification`,
  the four `update_*` tools and `set_task_repetition`. None is open-world. The full table is in
  [Tool annotations](docs/tools.md#tool-annotations).

### Changed
- An error raised inside OmniFocus is reported with its own message, without the
  `OmniFocus operation failed: ` prefix that every message outside `Task`/`Project`/`Tag`/`Folder
  not found:` used to get (for example `Parent task not found: <id>`,
  `Task is not completed: <id>`).

### Fixed
- `list_notifications` and `add_notification` report a due-relative notification as
  `kind: "relative"` with its `relativeFireOffset`. The kind was inferred from `initialFireDate`,
  which OmniFocus sets for relative notifications too, so every one read `kind: "absolute"` with
  `relativeFireOffset: null`. Any other kind, such as OmniFocus's invalid-state `Unknown`, is
  reported as `kind: "unknown"` with neither fire-date field read.
- `move_task` and `move_tasks_batch` now refuse to move a task under its own child or grandchild
  (`Cannot move a task under its own descendant.`); only a move under the task itself was refused.
  Both read the ancestor chain through `containingTask`, which OmniJS does not have, instead of
  `parent`, which also left the post-move "still nested under a parent" check unable to fire.
- `get_project` and `list_projects` no longer report a project's own root task as its next task
  (`nextTaskId`/`nextTaskName` named the project itself when no child was next, for example once
  it was completed); such a project reports `null`, and stall detection treats it the same way.
- `get_project` and `create_project` report `modified` from the project's root
  task; OmniJS projects have no `modified` of their own, so it was always `null`. A project created
  in the same call still reports `null`, as OmniFocus sets the date when it saves.
- A task's `completed` field now follows the `completed` status filter: true when the task is done
  directly, through a containing task, or through a completed project. A task left open when its
  project was completed read `completed: false` in every tool while the filter and the project's
  counts called it completed. `uncomplete_task` on such a task now names the project or task to
  reopen instead of failing with `Task is not completed`.
- `get_inbox` and `list_subtasks` return the same task summary as `list_tasks`. `list_subtasks`
  reported `projectName`, `plannedDate` and `completionDate` as `null`, and `get_inbox`
  `plannedDate`, whatever the task held.
- `create_project` and `create_tag` return the same object shape as `get_project` and a `list_tags`
  tag respectively, instead of only an id.
- Removed the duplicate `uncomplete_task` and `append_to_note` from `rust/src/tools/tasks.rs`; the
  server and the `smoke_test` example now use the single copies in `rust/src/tools/utility.rs`.
- Tag-valued writes now resolve each supplied id or exact name before changing anything and fail
  with `Tag not found: <value>` instead of silently skipping an unknown tag.
- Folder status output now compares `Folder.Status` enum members directly and reports an
  unrecognised value as `unknown` instead of `active`; `get_folder` still normalises its child
  projects with the separate project-status helper.
- `list_tasks`, `search_tasks` and `get_task_counts` now reject planned-date filters that an older,
  unmigrated database cannot honour. Planned-date sorting fails for the same reason instead of
  silently leaving the order unchanged.
- `create_tasks_batch` now resolves and validates every entry's project, tags and dates before it
  creates any task, and validation errors identify the failing entry's index.
- Task summaries now report each task's real inbox and sequential-action-group state instead of
  silently defaulting both fields to `false`.
- The `daily_review` prompt now excludes completed and otherwise non-remaining tasks from its
  flagged-task section.
- The `delete_tag` tool description now explains the non-destructive alternative and requires
  explicit user confirmation before deletion.
- Unrecognised `osascript` failures now carry one `JXA execution failed:` prefix instead of two.
- Task status filters and open-task counts now use OmniFocus's effective state: `available`
  excludes blocked, future-deferred and on-hold work, while `overdue`, `due_soon`, forecast and
  remaining counts exclude completed and dropped tasks and tasks in completed or dropped projects.
- Project root tasks exposed by `document.flattenedTasks` are no longer returned, found by task id,
  or included in task, project, tag and forecast counts.
- Active single-action lists are reported as stalled only when they have remaining tasks but none
  is available; their always-null `nextTask` no longer makes every such list stalled.
- Completed task counts include tasks completed through a containing task or completed project,
  while dropped tasks and tasks in dropped projects remain excluded.
- A script result that parses but does not have the shape a tool reads (a missing field, a value
  of the wrong type) now fails with `OmniFocus returned a result in an unexpected shape: <detail>`,
  naming the field, instead of `JXA command returned malformed JSON.`. Output that is not JSON at
  all still reports `JXA command returned malformed JSON`, now followed by the parser's detail.
- An error raised inside OmniFocus whose text contained words such as `Not permitted`,
  `Apple Events` or `not running` (often from user input, such as a tag name) was reported as
  `macOS blocked Automation access to OmniFocus` or `OmniFocus is not running`. Those two messages
  now come only from `osascript`'s own failure output.

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
  `move_tasks_batch`. Once a value is resolved to one project or folder, filters compare by its id,
  so the contents of another project or folder with the same name are no longer mixed in. A name
  shared by several projects or folders still selects the first; pass the id to choose.
  ([upstream #11](https://github.com/vitalyrodnenko/OmnifocusMCP/issues/11))
- A reference of every tool and parameter: [`docs/tools.md`](docs/tools.md).

### Changed
- **Breaking:** tool and prompt parameters reject unknown keys instead of ignoring them. A call
  with a misspelled or undeclared key (for example `folderId` where the tool declares `folder`)
  fails with an `invalid_params` error instead of succeeding without its argument. Every tool
  schema now advertises `additionalProperties: false`.
- **Breaking:** a bare `YYYY-MM-DD` date is read as a day in your local time zone, not as UTC
  midnight, so outside UTC tasks are no longer deferred hours late or to the wrong day. Written to
  `dueDate` or `deferDate` (tasks and projects), it gets the default due or start time set in
  OmniFocus, as the OmniFocus app does (17:00 and 00:00 out of the box; the server also uses these
  if it cannot read the setting). In filters (`dueBefore`, `completedAfter`, ...) and in
  `add_notification`'s `absoluteDate` it is local midnight. Date-times with `Z` or an offset are
  unchanged. ([upstream #13](https://github.com/vitalyrodnenko/OmnifocusMCP/issues/13))
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
- Tool descriptions, which the assistant reads to fill in parameters, state the 2.0.0 date rule:
  in `list_tasks`, `search_tasks`, `get_task_counts`, `list_projects` and `add_notification` a
  bare date is local midnight; in `create_task`, `create_subtask`, `update_task`, `create_project`
  and `update_project` it gets the OmniFocus default time. The `create_task` and `create_folder`
  descriptions say that `project` and `parent` take an id or an exact name.

### Fixed
- Integer, number and boolean parameters accept their string encoding (`"30"`, `"true"`), as sent
  by MCP clients that serialize every argument as a string. Integral floats (`30.0`) are accepted
  for integers. The advertised schema is unchanged.
  ([upstream #8](https://github.com/vitalyrodnenko/OmnifocusMCP/issues/8),
  [#11](https://github.com/vitalyrodnenko/OmnifocusMCP/issues/11))
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
