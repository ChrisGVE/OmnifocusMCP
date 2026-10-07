# Tool reference

This page lists every tool, resource and prompt that `omnifocus-mcp` 2.0.0 exposes, with the exact
parameter keys it accepts. The schemas your MCP client receives from `tools/list` are generated from
the same code and remain the authoritative source; this page adds the accepted values, defaults and
behaviour that a JSON schema cannot express.

For installation see [Installation](install-rust.md). For an overview see the
[README](../README.md).

## Contents

- [Conventions](#conventions)
- [Tasks (22 tools)](#tasks-22-tools)
- [Projects (12 tools)](#projects-12-tools)
- [Tags (6 tools)](#tags-6-tools)
- [Folders (6 tools)](#folders-6-tools)
- [Forecast (1 tool)](#forecast-1-tool)
- [Perspectives (1 tool)](#perspectives-1-tool)
- [Resources](#resources)
- [Prompts](#prompts)

## Conventions

### Parameter names

Keys are case-sensitive, and two spellings are in use:

- **snake_case only**: every key that takes an id, the three keys `object_type`, `rule_string`
  and `schedule_type`, and four date filters. The complete list: `task_id`, `task_ids`,
  `parent_task_id`, `notification_id`, `object_type`, `object_id`, `rule_string`, `schedule_type`,
  `project_id_or_name`, `project_ids_or_names`, `tag_name_or_id`, `tag_ids_or_names`,
  `folder_name_or_id`, `folder_ids_or_names`, and the four filters `added_after`, `added_before`,
  `changed_after`, `changed_before`.
- **camelCase, with a snake_case alias**: every other multi-word key, for example `dueDate`
  (also `due_date`), `dueBefore` (also `due_before`), `sortBy` (also `sort_by`). The schema
  advertises only the camelCase spelling; the snake_case alias is accepted but not listed.

Single-word keys (`name`, `note`, `project`, `folder`, `tag`, `tags`, `flagged`, `status`,
`limit`, `query`, `parent`, `sequential`, `text`, `tasks`) have one spelling.

### Reading the tables

Each tool's parameters are listed in a table with five columns:

- **Key**: the exact parameter key. Several keys in one cell share the other four columns.
- **Type**: the JSON type. `date` is a string in one of the forms under [Dates](#dates); the
  schema calls it `string`.
- **Required**: `yes` or `no`. A call without a required key fails with `invalid_params`.
- **Default**: what the tool uses when the key is omitted. A dash means there is none: a filter is
  not applied, and a field is left unset (or, in an update tool, unchanged).
- **Notes**: accepted values and anything the schema cannot express.

Each tool section starts with what the tool does and ends with what it returns.

### Unknown keys are rejected

Every tool schema sets `additionalProperties: false`. A key the tool does not declare, including a
misspelling such as `taskId` or `addedAfter`, fails the whole call with an `invalid_params` error
that names the unknown field. Nothing in OmniFocus changes.

### Scalar and tag encodings

Some MCP clients send every argument as a string. The server accepts both forms:

- `integer` parameters accept `30`, `"30"` and the integral float `30.0`.
- `number` parameters accept `-3600` and `"-3600"`.
- `boolean` parameters accept `true`/`false` and the strings `"true"`/`"false"` (any letter case).
  No other string (`"yes"`, `"1"`) is accepted.
- `tags` accepts an array of tag names (`["Home", "Quick"]`) or one string holding a JSON array
  (`"[\"Home\",\"Quick\"]"`).

### Dates

Date parameters take a bare date (`YYYY-MM-DD`) or an ISO 8601 date-time.

| Input | Read as |
| --- | --- |
| `2026-10-10` written to `dueDate` | 10 October, local time, at the default due time set in OmniFocus (17:00 out of the box; the server also uses 17:00 if it cannot read the setting) |
| `2026-10-10` written to `deferDate` | 10 October, local time, at the default start time set in OmniFocus (00:00 out of the box; the server also uses 00:00 if it cannot read the setting) |
| `2026-10-10` in a filter (`dueBefore`, `added_after`, ...) or in `absoluteDate` | 10 October, local midnight |
| `2026-10-10T09:30:00Z` or `2026-10-10T09:30:00+02:00` | That exact instant |
| `2026-10-10T09:30:00` (no offset) | 09:30 local time |

An impossible or unparseable date fails before anything is created or changed. Dates are checked
inside OmniFocus, so the error is an `internal_error` with the message
`OmniFocus operation failed: <field> must be YYYY-MM-DD or an ISO 8601 date-time; received "<value>"`.
In `create_tasks_batch` the field is named with its position, for example `tasks[2].dueDate`.

Filter bounds on due, defer, completion and planned dates are exclusive: `dueBefore: "2026-10-10"`
matches tasks due before local midnight at the start of 10 October. Bounds on `added_*` and
`changed_*` are inclusive. A task with no value for the filtered date never matches that filter.

### Id-or-name values

These parameters accept an OmniFocus id or an exact name: `project` and `folder` wherever they
appear, `create_folder`'s `parent`, and the keys `project_id_or_name`, `project_ids_or_names`,
`folder_name_or_id`, `folder_ids_or_names`, `tag_name_or_id` and `tag_ids_or_names`. If several
objects share a name, the first match in OmniFocus order is used; pass the id to choose.

A value that matches nothing is an error (`Project not found: <value>`,
`Folder not found: <value>`, `Tag not found: <value>`), never an empty result. The batch deletes
are the exception: they report such an entry in their results and go on with the others.

`create_tag`'s `parent` takes a name only, and `append_to_note`'s `object_id` an id only.

### Limits

Where a tool takes `limit`, the default is 100 and the value must be at least 1
(`limit must be greater than 0.`).

### Errors

| Kind | JSON-RPC error | Example message |
| --- | --- | --- |
| Invalid parameter (checked before OmniFocus is contacted) | `invalid_params` | `status must be one of: available, due_soon, overdue, on_hold, completed, all. received: "remaining".` |
| Invalid date (checked inside OmniFocus) | `internal_error` | `OmniFocus operation failed: dueDate must be YYYY-MM-DD or an ISO 8601 date-time; received "2026-02-30"` |
| Object not found | `internal_error` | `Task not found: <id>`, `Project not found: <value>`, `Folder not found: <value>`, `Tag not found: <value>` |
| Other error raised inside OmniFocus | `internal_error` | `OmniFocus operation failed: <message>` |
| OmniFocus not running | `internal_error` | `JXA execution failed: OmniFocus is not running. Please open OmniFocus and try again.` |
| Automation permission missing | `internal_error` | `JXA execution failed: macOS blocked Automation access to OmniFocus. Grant permission in System Settings > Privacy & Security > Automation.` |
| Call took longer than 30 seconds | `internal_error` | `JXA command timed out after 30s.` |

The server runs one OmniFocus script at a time. Concurrent calls wait for each other.

### Task summaries

`get_inbox`, `list_tasks`, `search_tasks` and `list_subtasks` return arrays of task summaries with
these fields: `id`, `name`, `note`, `flagged`, `completed`, `projectName`, `dueDate`, `deferDate`,
`completionDate`, `plannedDate`, `addedDate`, `changedDate` (OmniFocus `modified`), `tags` (names),
`estimatedMinutes`, `hasChildren`, `taskStatus`, `inInbox`, `sequential`. Dates are ISO 8601 in
UTC, or `null`.

Not every tool fills every field. A field a tool does not fill is always `null` (`false` for
`inInbox` and `sequential`), whatever the task holds, so it must not be read as "no value":

| Tool | Fields always `null` or `false` |
| --- | --- |
| `get_inbox` | `projectName`, `plannedDate`, `inInbox`, `sequential` |
| `list_tasks`, `search_tasks` | `inInbox`, `sequential` |
| `list_subtasks` | `projectName`, `completionDate`, `plannedDate`, `inInbox`, `sequential` |

`get_task` returns the real values of all of these except `inInbox`.

`taskStatus` is OmniFocus's own status of the task: `available`, `blocked`, `next`, `due_soon`,
`overdue`, `completed`, `dropped`, or `unknown`.

## Tasks (22 tools)

### Task filters

`list_tasks`, `search_tasks` and `get_task_counts` share these filters.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `project` | string | no | - | Project id or exact name. Only tasks in that project. |
| `tag` | string | no | - | One tag name. |
| `tags` | array of strings | no | - | Tag names. Merged with `tag`. |
| `tagFilterMode` | string | no | `any` | `any`: the task has at least one of the tags. `all`: it has every one. Aliases `or` and `and`, any letter case. |
| `flagged` | boolean | no | - | `true`: flagged tasks only. `false`: unflagged tasks only. Omit for both. |
| `dueBefore`, `dueAfter` | date | no | - | Due date range. |
| `deferBefore`, `deferAfter` | date | no | - | Defer date range. |
| `completedBefore`, `completedAfter` | date | no | - | Completion date range. |
| `added_before`, `added_after` | date | no | - | Creation date range. snake_case only. |
| `changed_before`, `changed_after` | date | no | - | Last-modified date range (OmniFocus `modified`). snake_case only. |
| `plannedBefore`, `plannedAfter` | date | no | - | Planned date range. Ignored on an OmniFocus version without planned dates. |
| `maxEstimatedMinutes` | integer | no | - | At least 0. Tasks with an estimate of at most this many minutes. Tasks without an estimate are excluded. |

Tags are matched by name.

`list_tasks` and `search_tasks` also take:

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `status` | string | no | `available` | See below. |
| `sortBy` | string | no | - | `name`, `dueDate`, `deferDate`, `completionDate`, `estimatedMinutes`, `project`, `flagged`, `addedDate`, `changedDate`, `plannedDate`, or the aliases `added`, `modified`, `planned`. Case-sensitive. Tasks without a value sort last. Without `sortBy`, tasks keep OmniFocus order. |
| `sortOrder` | string | no | `asc` | `asc` or `desc`. Aliases `ascending`, `descending`, any letter case. |
| `limit` | integer | no | 100 | Maximum number of tasks returned. |

`status` values (any letter case; `-` or a space may replace `_`):

| Value | Returns |
| --- | --- |
| `available` | Every task not marked completed, including deferred tasks. |
| `due_soon` | Not completed, due between now and 7 days from now. |
| `overdue` | Not completed, due before now. |
| `on_hold` | Not completed, in a project whose status is on hold. |
| `completed` | Completed tasks. |
| `all` | Every task. |

`status` looks only at the task's own completed flag. See
[Known limitations](../README.md#known-limitations) for what that includes.

When `completedBefore` or `completedAfter` is set, the result contains completed tasks whatever
`status` says, and without a `sortBy` it is sorted by completion date, newest first.

### `get_inbox`

Lists incomplete tasks in the inbox.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `limit` | integer | no | 100 | Maximum number of tasks returned. |

Returns task summaries.

### `list_tasks`

Lists tasks across the whole database, filtered and sorted.

Parameters: the [task filters](#task-filters) plus `status`, `sortBy`, `sortOrder`, `limit`.
Returns task summaries.

### `search_tasks`

Like `list_tasks`, restricted to tasks whose name or note contains `query` (case-insensitive
substring match).

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `query` | string | yes | - | Not empty. |

Plus the [task filters](#task-filters) and `status`, `sortBy`, `sortOrder`, `limit`. Returns task
summaries.

### `get_task_counts`

Counts tasks matching the [task filters](#task-filters) in one call, without returning the tasks.
It takes no `status`, `sortBy`, `sortOrder` or `limit`.

Returns `{total, available, completed, overdue, dueSoon, flagged, deferred}`:

- `total`: every matching task, completed or not.
- `completed`: matching tasks marked completed.
- `flagged`: matching flagged tasks, completed or not.
- `available`: not completed, and with no defer date or a defer date in the past.
- `deferred`: not completed, with a defer date in the future.
- `overdue`: not completed, due before now.
- `dueSoon`: not completed, due between now and 7 days from now.

`available` here excludes deferred tasks; `status: "available"` in `list_tasks` does not.

### `get_task`

Reads one task in full.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |

Returns the task with the summary fields except `hasChildren` and `inInbox`, plus
`effectiveDueDate`, `effectiveDeferDate`, `effectiveFlagged`, `effectivePlannedDate`, `modified`
(the same value as `changedDate`), `children` (`id`, `name`, `completed` of each direct child),
`parentName`, and `repetitionRule` (the rule string, or `null`).

### `list_subtasks`

Lists the direct children of a task.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |
| `limit` | integer | no | 100 | Maximum number of tasks returned. |

Returns task summaries.

### `create_task`

Creates one task.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `name` | string | yes | - | Not empty. |
| `project` | string | no | inbox | Project id or exact name. |
| `note` | string | no | - | |
| `dueDate`, `deferDate` | date | no | - | |
| `flagged` | boolean | no | - | |
| `tags` | array of strings | no | - | Names of existing tags. A name that matches no tag is skipped without an error. |
| `estimatedMinutes` | integer | no | - | |

Returns `{id, name}`.

### `create_tasks_batch`

Creates several tasks in one OmniFocus call.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `tasks` | array of objects | yes | - | At least one. Each object takes the keys of `create_task`, with `name` required. |

Every project and every date in the batch is checked before the first task is created, so one bad
entry creates nothing. Returns an array of `{id, name}`.

### `create_subtask`

Creates a task under an existing task.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `name` | string | yes | - | Not empty. |
| `parent_task_id` | string | yes | - | Id of the parent task. |
| `note` | string | no | - | |
| `dueDate`, `deferDate` | date | no | - | |
| `flagged` | boolean | no | - | |
| `tags` | array of strings | no | - | As in `create_task`. |
| `estimatedMinutes` | integer | no | - | |

Returns `{id, name, parentTaskId, parentTaskName}`.

### `update_task`

Changes only the fields you pass.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |
| `name` | string | no | - | Not empty when given. |
| `note` | string | no | - | Replaces the whole note (see `append_to_note`). |
| `dueDate`, `deferDate` | date | no | - | |
| `flagged` | boolean | no | - | |
| `tags` | array of strings | no | - | Replaces all tags. Unknown names are skipped. |
| `estimatedMinutes` | integer | no | - | |

Omitting a field and passing `null` both leave it unchanged, so this tool cannot clear a date.
Returns the updated task, including effective dates.

### `complete_task`

Marks a task completed.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |

Returns `{id, name, completed}`.

### `uncomplete_task`

Reopens a completed task.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |

On a task that is not completed it fails with
`OmniFocus operation failed: Task is not completed: <id>`. Returns `{id, name, completed}`.

### `set_task_repetition`

Sets or removes a task's repetition.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |
| `rule_string` | string | no | - | An iCalendar recurrence rule such as `FREQ=WEEKLY;INTERVAL=1`. Omit it or pass `null` to remove the repetition. |
| `schedule_type` | string | no | `regularly` | `regularly`, `from_completion`, or `none`. |

Returns `{id, name, repetitionRule}`.

### `duplicate_task`

Copies a task.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |
| `includeChildren` | boolean | no | `true` | Copy the subtasks too. |

Creates the copy at the end of the task's project, or in the inbox when the task has no project;
a copied subtask is not placed under the original's parent. Returns the new task.

### `move_task`

Moves a task without deleting it, so its id and history are kept.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |
| `project` | string | no | - | Project id or exact name. |
| `parent_task_id` | string | no | - | Id of the new parent task. |

Pass `project` to move to the top level of a project, `parent_task_id` to make the task a
subtask, or neither to move it to the inbox. Passing both fails with
`provide either project or parent_task_id, not both (destination is ambiguous).` A task cannot be
moved under itself or its own descendant. Returns `{id, name, projectName, inInbox}`.

### `move_tasks_batch`

Moves several tasks to one destination.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_ids` | array of strings | yes | - | At least one, no duplicates. |
| `project` | string | no | - | As in `move_task`. |
| `parent_task_id` | string | no | - | As in `move_task`. Must not be one of `task_ids`. |

Returns `{requested_count, moved_count, failed_count, partial_success, results}`, with one result
per id.

### `delete_task`

Deletes a task and its subtasks immediately.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |

Returns `{id, name, deleted, warning}`; `warning` gives the number of child tasks when there were
any.

### `delete_tasks_batch`

Deletes several tasks immediately.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_ids` | array of strings | yes | - | At least one, no duplicates. |

Returns `{deleted_count, not_found_count, results}`. An id that matches no task is reported in
`results` and does not stop the others.

### `append_to_note`

Appends text to the note of a task or a project, keeping the existing note.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `object_type` | string | yes | - | `task` or `project`. |
| `object_id` | string | yes | - | An id; names are not accepted. |
| `text` | string | yes | - | Not empty. |

Returns `{id, name, type, noteLength}`.

### `list_notifications`

Lists a task's notifications.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |

Returns `id`, `kind` (`absolute` or `relative`), `absoluteFireDate`, `relativeFireOffset`,
`nextFireDate` and `isSnoozed` for each notification.

### `add_notification`

Adds a notification to a task.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |
| `absoluteDate` | date | no | - | Pass exactly one of `absoluteDate` and `relativeOffset`. |
| `relativeOffset` | number | no | - | Seconds relative to the task's effective due date; `-3600` is one hour before. |

A relative notification needs a task with an effective due date. Returns the created notification.

### `remove_notification`

Removes one notification from a task.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |
| `notification_id` | string | yes | - | |

Returns `{taskId, notificationId, removed}`.

## Projects (12 tools)

Project status values are `active`, `on_hold`, `completed` and `dropped`. A status the server does
not recognise is reported as `unknown`.

A project is **stalled** when it is active, has at least one incomplete task, and OmniFocus reports
no next task for it. Dropped tasks count as incomplete here, as in every project task count; see
[Known limitations](../README.md#known-limitations).

### `list_projects`

Lists projects, filtered and sorted.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `folder` | string | no | - | Folder id or exact name. Only projects directly in that folder. |
| `status` | string | no | `active` | `active`, `on_hold`, `completed`, or `dropped`. Exact values; there is no `all`. |
| `completedBefore`, `completedAfter` | date | no | - | Completion date range. Setting either one lists completed projects, whatever `status` says. |
| `stalledOnly` | boolean | no | `false` | When `true`, lists stalled projects only. |
| `sortBy` | string | no | - | `name`, `dueDate`, `completionDate`, or `taskCount`. |
| `sortOrder` | string | no | `asc` | `asc` or `desc`. No aliases. |
| `limit` | integer | no | 100 | Maximum number of projects returned. |

Returns projects with `id`, `name`, `status`, `folderName`, `taskCount`, `remainingTaskCount`
(tasks not marked completed, dropped ones included), `deferDate`, `dueDate`, `completionDate`,
`note`, `sequential`, `isStalled`, `nextTaskId`, `nextTaskName`, `reviewInterval`.
`reviewInterval` is text such as `"2 weeks"` or `"1 month"`, or `null`.

### `get_project_counts`

Counts projects by status in one call.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `folder` | string | no | - | Folder id or exact name. Only projects directly in that folder. |

Returns `{total, active, onHold, completed, dropped, stalled}`.

### `search_projects`

Finds projects by name.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `query` | string | yes | - | Not empty. |
| `limit` | integer | no | 100 | Maximum number of projects returned. |

Uses OmniFocus's own project search, so matching follows OmniFocus rules rather than a plain
substring test. Returns `{id, name, status, folderName}` per project.

### `get_project`

Reads one project in full.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `project_id_or_name` | string | yes | - | |

Returns the `list_projects` fields plus `completedTaskCount`, `availableTaskCount` (tasks not
marked completed and not deferred to the future, dropped ones included), `modified` (the
last-modified date) and `rootTasks` (the project's top-level tasks).

### `create_project`

Creates one project.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `name` | string | yes | - | Not empty. |
| `folder` | string | no | top level | Folder id or exact name. |
| `note` | string | no | - | |
| `dueDate`, `deferDate` | date | no | - | |
| `sequential` | boolean | no | - | |

Returns `{id}`.

### `update_project`

Changes only the fields you pass.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `project_id_or_name` | string | yes | - | |
| `name` | string | no | - | Not empty when given. |
| `note` | string | no | - | Replaces the whole note. |
| `dueDate`, `deferDate` | date | no | - | |
| `flagged` | boolean | no | - | |
| `tags` | array of strings | no | - | Replaces all tags. Unknown names are skipped. |
| `sequential` | boolean | no | - | |
| `completedByChildren` | boolean | no | - | |
| `reviewInterval` | string | no | - | `"N unit"`, N a whole number of at least 1, unit `day(s)`, `week(s)`, `month(s)` or `year(s)` in any letter case, e.g. `"2 weeks"`. |

`reviewInterval` can only change an interval the project already has; on a project without one
the call fails with `OmniFocus operation failed: Project has no review interval to update` and
nothing is modified. A malformed value fails before OmniFocus is contacted. As in `update_task`,
this tool cannot clear a date. Returns the updated project.

### `set_project_status`

Sets a project to active, on hold or dropped.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `project_id_or_name` | string | yes | - | |
| `status` | string | yes | - | `active`, `on_hold`, or `dropped`. Exact values. |

To mark a project finished use `complete_project`, not `dropped`. Returns `{id, name, status}`.

### `complete_project`

Marks a project completed.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `project_id_or_name` | string | yes | - | |

Returns `{id, name, completed}`.

### `uncomplete_project`

Reopens a completed project.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `project_id_or_name` | string | yes | - | |

On a project that is not completed it fails with
`OmniFocus operation failed: Project is not completed: <value>`. Returns `{id, name, status}`.

### `move_project`

Moves a project to another folder or to the top level.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `project_id_or_name` | string | yes | - | |
| `folder` | string | no | top level | Folder id or exact name. |

Returns `{id, name, folderName}`, where `folderName` is the folder the project is in after the move.

### `delete_project`

Deletes a project and all its tasks immediately.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `project_id_or_name` | string | yes | - | |

Returns `{id, name, deleted, taskCount}`.

### `delete_projects_batch`

Deletes several projects and their tasks immediately.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `project_ids_or_names` | array of strings | yes | - | At least one, no duplicates. |

Returns `{summary: {requested, deleted, failed}, partial_success, results}`; an entry that matches
nothing is reported as `not found` and does not stop the others.

## Tags (6 tools)

Tag status values are `active`, `on_hold` and `dropped`.

### `list_tags`

Lists tags, filtered by status and sorted.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `statusFilter` | string | no | `all` | `all`, `active`, `on_hold`, or `dropped`. |
| `sortBy` | string | no | - | `name`, `availableTaskCount`, or `totalTaskCount`. |
| `sortOrder` | string | no | `asc` | `asc` or `desc`. No aliases. |
| `limit` | integer | no | 100 | Maximum number of tags returned. |

Returns `{id, name, parent, availableTaskCount, totalTaskCount, status}` per tag.
`totalTaskCount` counts every task with the tag; `availableTaskCount` counts those not marked
completed, dropped and deferred ones included.

### `search_tags`

Finds tags by name.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `query` | string | yes | - | Not empty. |
| `limit` | integer | no | 100 | Maximum number of tags returned. |

Uses OmniFocus's own tag search. Returns `{id, name, status, parent}` per tag.

### `create_tag`

Creates one tag.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `name` | string | yes | - | Not empty. |
| `parent` | string | no | top level | Name of an existing tag; ids are not accepted. |

Returns `{id}`.

### `update_tag`

Renames a tag or changes its status.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `tag_name_or_id` | string | yes | - | |
| `name` | string | no | - | |
| `status` | string | no | - | `active`, `on_hold`, or `dropped`. |

At least one of `name` and `status` is required. Returns `{id, name, status}`.

### `delete_tag`

Deletes a tag immediately; tasks keep existing but lose the tag.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `tag_name_or_id` | string | yes | - | |

Returns `{id, name, deleted, taskCount}`.

### `delete_tags_batch`

Deletes several tags immediately.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `tag_ids_or_names` | array of strings | yes | - | At least one, no duplicates. |

Returns `{summary: {requested, deleted, failed}, partial_success, results}`.

## Folders (6 tools)

Folder status values are `active` and `dropped`.

### `list_folders`

Lists every folder at any depth.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `limit` | integer | no | 100 | Maximum number of folders returned. |

Returns `{id, name, parentName, projectCount}` per folder, where `projectCount` counts projects
directly in the folder.

### `get_folder`

Reads one folder and its direct contents.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `folder_name_or_id` | string | yes | - | |

Returns `{id, name, status, parentName, projects, subfolders}` with the folder's direct projects
(`id`, `name`, `status`) and direct subfolders (`id`, `name`).

### `create_folder`

Creates one folder.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `name` | string | yes | - | Not empty. |
| `parent` | string | no | top level | Folder id or exact name. |

Returns `{id, name}`.

### `update_folder`

Renames a folder or changes its status.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `folder_name_or_id` | string | yes | - | |
| `name` | string | no | - | |
| `status` | string | no | - | `active` or `dropped`. |

At least one of `name` and `status` is required. Returns `{id, name, status}`.

### `delete_folder`

Deletes a folder immediately. What happens to the folder's projects is up to OmniFocus.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `folder_name_or_id` | string | yes | - | |

Returns `{id, name, deleted, projectCount, subfolderCount}`.

### `delete_folders_batch`

Deletes several folders immediately.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `folder_ids_or_names` | array of strings | yes | - | At least one, no duplicates. |

Returns `{summary: {requested, deleted, failed}, partial_success, results}`.

## Forecast (1 tool)

### `get_forecast`

Looks at tasks not marked completed and returns five sections plus their full counts.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `limit` | integer | no | 100 | Applied to each section. |

The sections:

| Section | Tasks |
| --- | --- |
| `overdue` | due before the start of today |
| `dueToday` | due today |
| `flagged` | flagged |
| `deferred` | defer date in the future |
| `dueThisWeek` | due after today and within 7 days from now |

`counts` holds `overdueCount`, `dueTodayCount`, `flaggedCount`, `deferredCount` and
`dueThisWeekCount`, which are not capped by `limit`. A task can appear in more than one section.
Note that `overdue` here starts at midnight, while `status: "overdue"` in `list_tasks` uses the
current time.

## Perspectives (1 tool)

### `list_perspectives`

Lists built-in and custom perspectives, without duplicates.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `limit` | integer | no | 100 | Maximum number of perspectives returned. |

Returns `{id, name}` per perspective. No tool reads the contents of a perspective.

## Resources

| URI | Name | Content |
| --- | --- | --- |
| `omnifocus://inbox` | Inbox tasks | `get_inbox` with `limit` 100 |
| `omnifocus://today` | Today forecast | `get_forecast` with `limit` 100 (all five sections) |
| `omnifocus://projects` | Active projects | `list_projects` with `status: "active"` and `limit` 100 |

Each resource is JSON text, read from OmniFocus when the client requests it.

## Prompts

| Name | Arguments | What it sends |
| --- | --- | --- |
| `daily_review` | none | Up to 25 tasks each that are due soon, overdue, and flagged, with instructions for a daily plan. The flagged list is the first 25 flagged tasks in OmniFocus order, completed or not, so completed tasks can crowd out current ones |
| `weekly_review` | none | Up to 500 active projects and 1,000 available tasks, with instructions for a weekly review |
| `inbox_processing` | none | Up to 200 inbox tasks, with instructions to clarify them one by one |
| `project_planning` | `project` (required): project id or exact name | The project's details and available tasks, with planning instructions |

If `project_planning` is given a project that does not exist, it still returns a prompt; the
project appears in it with status `not_found` and no tasks.

Each prompt asks the assistant to propose changes and to ask before applying them.
`daily_review`, `weekly_review` and `inbox_processing` also ask it to get explicit confirmation
before any delete; `project_planning` proposes new work and does not mention deletes.
