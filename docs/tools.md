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
- `tags` accepts an array of tag ids or exact names (`["Home", "Quick"]`) or one string holding a JSON array
  (`"[\"Home\",\"Quick\"]"`). An empty or blank string (`""`) is rejected with `invalid_params`
  (`tags must not be an empty string; pass [] to remove every tag`) on every tool that takes `tags`,
  so a stray empty value can never clear a task's or project's tags. `[]` (or `"[]"`) is the
  explicit empty list.

### Dates

Date parameters take a bare date (`YYYY-MM-DD`) or an ISO 8601 date-time.

| Input | Read as |
| --- | --- |
| `2026-10-10` written to `dueDate` | 10 October, local time, at the default due time set in OmniFocus (17:00 out of the box; the server also uses 17:00 if it cannot read the setting) |
| `2026-10-10` written to `deferDate` | 10 October, local time, at the default start time set in OmniFocus (00:00 out of the box; the server also uses 00:00 if it cannot read the setting) |
| `2026-10-10` written to `plannedDate` | 10 October, local time, at the default planned time set in OmniFocus (the server uses 09:00 if it cannot read the setting) |
| `2026-10-10` in a filter (`dueBefore`, `added_after`, ...) or in `absoluteDate` | 10 October, local midnight |
| `2026-10-10T09:30:00Z` or `2026-10-10T09:30:00+02:00` | That exact instant |
| `2026-10-10T09:30:00` (no offset) | 09:30 local time |

An impossible or unparseable date fails before anything is created or changed. Dates are checked
inside OmniFocus, and the call fails (see [Errors](#errors)) with the message
`<field> must be YYYY-MM-DD or an ISO 8601 date-time; received "<value>"`.
In `create_tasks_batch` the field is named with its position, for example `tasks[2].dueDate`.

Filter bounds on due, defer, completion and planned dates are exclusive: `dueBefore: "2026-10-10"`
matches tasks due before local midnight at the start of 10 October. Bounds on `added_*` and
`changed_*` are inclusive. A task with no value for the filtered date never matches that filter.

### Id-or-name values

These parameters accept an OmniFocus id or an exact name: `project`, `folder`, and tag references
wherever they appear; `create_folder`'s and `create_tag`'s `parent`; and the keys
`project_id_or_name`, `project_ids_or_names`, `folder_name_or_id`, `folder_ids_or_names`,
`tag_name_or_id` and `tag_ids_or_names`. A value is tried as an id first, then as an exact name.

Names are not unique in OmniFocus, so a name that several objects of the same kind share is
refused, in reads and writes alike:
`Ambiguous project name "Errands": 2 matches (<id>, <id>); pass an id.` (`folder` and `tag` the
same way). The message lists the ids to choose from. An id always names exactly one object.

A value that matches nothing is an error (`Project not found: <value>`,
`Folder not found: <value>`, `Tag not found: <value>`), never an empty result. The batch deletes
are the exception: they report an entry that matches nothing (`"not found"`) or names several
objects (the `Ambiguous …` message) in its result, delete nothing for it, and go on with the
others.

`append_to_note`'s `object_id` takes an id only.

### Limits

Where a tool takes `limit`, the default is 100 and the value must be at least 1
(`limit must be greater than 0.`).

### Errors

A tool call fails in one of two ways, as the MCP specification separates them:

- **The request is rejected** with a JSON-RPC error, code `-32602` (`invalid_params`), before the
  tool runs: the tool name is unknown, or the arguments do not fit the tool's schema (an
  undeclared key, a missing required key, a value of the wrong JSON type).
- **The tool runs and fails.** The response is a normal tool result with `isError: true` whose
  single text item is the message. Clients pass these on to the model, so it can see what went
  wrong and correct the call.

| Kind | Reported as | Example message |
| --- | --- | --- |
| Unknown tool, undeclared or missing key (except `set_task_repetition`'s `rule_string`, below), wrong JSON type | JSON-RPC `invalid_params` | `tool not found`; for arguments, a `failed to deserialize parameters:` message naming the field |
| Invalid parameter value, or a missing `rule_string` on `set_task_repetition` (checked before OmniFocus is contacted) | `isError` result | `status must be one of: available, due_soon, overdue, on_hold, completed, all. received: "remaining".` |
| Invalid date (checked inside OmniFocus) | `isError` result | `dueDate must be YYYY-MM-DD or an ISO 8601 date-time; received "2026-02-30"` |
| Object not found | `isError` result | `Task not found: <id>`, `Project not found: <value>`, `Folder not found: <value>`, `Tag not found: <value>` |
| Other error raised inside OmniFocus | `isError` result | The script's own message, unchanged, e.g. `Parent task not found: <id>` |
| OmniFocus not running | `isError` result | `JXA execution failed: OmniFocus is not running. Please open OmniFocus and try again.` |
| Automation permission missing | `isError` result | `JXA execution failed: macOS blocked Automation access to OmniFocus. Grant permission in System Settings > Privacy & Security > Automation.` |
| Call took longer than 30 seconds | `isError` result | `OmniFocus did not answer within 30s, so the outcome is unknown: a change this call makes may still be applied. Read the object back before retrying.` |

Resources and prompts have no `isError` result in MCP, so when one of them fails the response is a
JSON-RPC error with the same message: `invalid_params` for an invalid parameter value,
`internal_error` for anything else.

The server runs one OmniFocus script at a time. Concurrent calls wait for each other.

### Tool annotations

Every tool states the four MCP annotation hints in `tools/list`, so a client can ask before a call
that loses data without asking before every read. `openWorldHint` is `false` for all tools: they
work only on the local OmniFocus database. A client that does not read annotations sees no
difference.

Destructive means the tool can overwrite or remove existing content. That covers the deletes and
also the update tools and `set_task_repetition`, which replace values you entered (`update_task`
replaces the note, and its `tags` replaces the task's tags). Creates, appends, moves, completing
or reopening, and `set_project_status` only add content or change a state you can change back.

| Kind | `readOnlyHint` | `destructiveHint` | `idempotentHint` | Tools |
| --- | --- | --- | --- | --- |
| Read | `true` | `false` | `true` | `get_inbox`, `list_tasks`, `get_task_counts`, `get_task`, `list_subtasks`, `search_tasks`, `list_notifications`, `list_projects`, `get_project_counts`, `search_projects`, `get_project`, `search_tags`, `list_tags`, `list_folders`, `get_folder`, `get_forecast`, `list_perspectives` |
| Change, safe to repeat | `false` | `false` | `true` | `uncomplete_task`, `move_task`, `move_tasks_batch`, `uncomplete_project`, `move_project`, `set_project_status` |
| Change | `false` | `false` | `false` | `create_task`, `create_subtask`, `create_tasks_batch`, `duplicate_task`, `complete_task`, `append_to_note`, `add_notification`, `create_project`, `complete_project`, `create_tag`, `create_folder` |
| Destructive, safe to repeat | `false` | `true` | `true` | `update_task`, `set_task_repetition`, `delete_task`, `delete_tasks_batch`, `remove_notification` |
| Destructive | `false` | `true` | `false` | `update_project`, `update_tag`, `update_folder`, `delete_project`, `delete_projects_batch`, `delete_tag`, `delete_tags_batch`, `delete_folder`, `delete_folders_batch` |

A tool is marked safe to repeat only when calling it again with the same arguments changes nothing
more. These are not:

- `complete_task` and `complete_project`: completing a repeating item advances it to its next
  occurrence, so a second call completes that occurrence too.
- `create_*`, `duplicate_task`, `add_notification`, `append_to_note`: each call adds again.
- `update_project`, `update_tag`, `update_folder` and the project, tag and folder deletes keep a
  conservative `false`. They were marked when a name shared by two objects reached the first and a
  repeated call then reached the second; a shared name is now refused
  ([Id-or-name values](#id-or-name-values)), so repeating one of these calls changes nothing more.

### Task summaries

`get_inbox`, `list_tasks`, `search_tasks` and `list_subtasks` return arrays of task summaries with
these fields: `id`, `name`, `note`, `flagged`, `completed`, `projectName`, `dueDate`, `deferDate`,
`completionDate`, `plannedDate`, `addedDate`, `changedDate` (OmniFocus `modified`), `tags` (names),
`estimatedMinutes`, `hasChildren`, `taskStatus`, `inInbox`, `sequential`. Dates are ISO 8601 in
UTC, or `null`.

`completed` is true when the task is done directly, through a containing task, or through a
completed project, the same definition the `completed` status filter uses. A task left open when its
project was completed is reported `completed: true`, although OmniFocus keeps its own checkbox
clear. Every tool that returns a task's `completed` uses this definition. A dropped task is never
completed: it reads `completed: false` like an open task, and `taskStatus: "dropped"` is what tells
them apart, so a client checking whether a task is still to do reads both fields.

Every task-summary tool reports the task's live `inInbox` and `sequential` values. Some other
fields are unavailable on narrower summary surfaces:

| Tool | Fields unavailable (`null`) |
| --- | --- |
| `get_inbox` | `projectName`, `plannedDate` |
| `list_tasks`, `search_tasks` | none |
| `list_subtasks` | `projectName`, `completionDate`, `plannedDate` |

`get_task` returns the real values of all of these. Forecast task summaries, project `rootTasks`,
and rich task results from `duplicate_task` and `update_task` also include the live `inInbox` and
`sequential` values.

`taskStatus` is OmniFocus's own status of the task: `available`, `blocked`, `next`, `due_soon`,
`overdue`, `completed`, `dropped`, or `unknown`.

## Tasks (22 tools)

### Task filters

`list_tasks`, `search_tasks` and `get_task_counts` share these filters.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `project` | string | no | - | Project id or exact name. Only tasks in that project. |
| `tag` | string | no | - | One tag id or exact name. |
| `tags` | array of strings | no | - | Tag ids or exact names. Merged with `tag`. |
| `tagFilterMode` | string | no | `any` | `any`: the task has at least one of the tags. `all`: it has every one. Aliases `or` and `and`, any letter case. |
| `flagged` | boolean | no | - | `true`: flagged tasks only. `false`: unflagged tasks only. Omit for both. |
| `dueBefore`, `dueAfter` | date | no | - | Due date range. |
| `deferBefore`, `deferAfter` | date | no | - | Defer date range. |
| `completedBefore`, `completedAfter` | date | no | - | Completion date range. |
| `added_before`, `added_after` | date | no | - | Creation date range. snake_case only. |
| `changed_before`, `changed_after` | date | no | - | Last-modified date range (OmniFocus `modified`). snake_case only. |
| `plannedBefore`, `plannedAfter` | date | no | - | Planned date range. If the database does not support planned dates, a supplied bound fails instead of being ignored. |
| `maxEstimatedMinutes` | integer | no | - | At least 0. Tasks with an estimate of at most this many minutes. Tasks without an estimate are excluded. |

Each `tag` and `tags` value is resolved like any other [id-or-name value](#id-or-name-values): a
value that matches no tag, or a name several tags share, fails the call
(`Tag not found: <value>`, `Ambiguous tag name …`) instead of matching nothing. Tasks are then
matched by the tags those values resolve to.

`list_tasks` and `search_tasks` also take:

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `status` | string | no | `available` | See below. |
| `sortBy` | string | no | - | `name`, `dueDate`, `deferDate`, `completionDate`, `estimatedMinutes`, `project`, `flagged`, `addedDate`, `changedDate`, `plannedDate`, or the aliases `added`, `modified`, `planned`. Case-sensitive. Tasks without a value sort last. Without `sortBy`, tasks keep OmniFocus order. |
| `sortOrder` | string | no | `asc` | `asc` or `desc`. Aliases `ascending`, `descending`, any letter case. |
| `limit` | integer | no | 100 | Maximum number of tasks returned. |

On a database without planned-date support, `sortBy: "plannedDate"` and its `planned` alias fail
instead of silently leaving tasks in OmniFocus order.

`status` values (any letter case; `-` or a space may replace `_`):

| Value | Returns |
| --- | --- |
| `available` | Remaining tasks that OmniFocus reports as actionable (`Available`, `Next`, `DueSoon` or `Overdue`), in an active project or the inbox, not deferred into the future, and without an on-hold tag. |
| `due_soon` | Remaining tasks due between now and 7 days from now. A blocked task can still match by date. |
| `overdue` | Remaining tasks due before now. A blocked task can still match by date. |
| `on_hold` | Remaining tasks in a project whose status is on hold. |
| `completed` | Tasks completed directly, through a containing task, or through a completed project. Dropped tasks do not match. |
| `all` | Every action task, including completed and dropped tasks. |

Project root tasks, which OmniFocus includes in `document.flattenedTasks` as project objects rather
than actions, are excluded from every value.

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

- `total`: every matching action task, completed or not. Project root tasks are excluded.
- `completed`: tasks completed directly, through a containing task, or through a completed
  project. Dropped tasks and tasks in dropped projects are excluded.
- `flagged`: matching flagged tasks, completed or not.
- `available`: remaining and actionable now, using the same definition as `status: "available"`.
- `deferred`: remaining, with a defer date in the future.
- `overdue`: remaining, due before now.
- `dueSoon`: remaining, due between now and 7 days from now.

"Remaining" excludes completed and dropped tasks, including completion or dropping inherited from
a containing task or project. It includes blocked tasks and tasks in on-hold projects.

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
| `dueDate`, `deferDate`, `plannedDate` | date | no | - | `plannedDate` requires an OmniFocus database migrated for planned dates; on an older database a supplied value fails before anything changes. |
| `flagged` | boolean | no | - | |
| `tags` | array of strings | no | - | Ids or exact names of existing tags. An unknown value fails before the task is created. |
| `estimatedMinutes` | integer | no | - | At least 0; a negative value fails before OmniFocus is contacted (`estimatedMinutes must be greater than or equal to 0.`). |

Returns `{id, name, plannedDate}`.

### `create_tasks_batch`

Creates several tasks in one OmniFocus call.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `tasks` | array of objects | yes | - | At least one. Each object takes the keys of `create_task`, with `name` required. |

Every project, tag, date and `estimatedMinutes` in the batch is checked before the first task is
created, so one bad entry creates nothing. The error identifies the entry, for example
`tasks[2].project`, `tasks[2].tags[0]` or `tasks[2].estimatedMinutes`. Returns an array of `{id, name, plannedDate}`.

### `create_subtask`

Creates a task under an existing task.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `name` | string | yes | - | Not empty. |
| `parent_task_id` | string | yes | - | Id of the parent task. |
| `note` | string | no | - | |
| `dueDate`, `deferDate`, `plannedDate` | date | no | - | `plannedDate` requires an OmniFocus database migrated for planned dates. |
| `flagged` | boolean | no | - | |
| `tags` | array of strings | no | - | As in `create_task`. |
| `estimatedMinutes` | integer | no | - | At least 0; a negative value fails before OmniFocus is contacted (`estimatedMinutes must be greater than or equal to 0.`). |

Returns `{id, name, parentTaskId, parentTaskName, plannedDate}`.

### `update_task`

Changes only the fields you pass.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |
| `name` | string | no | - | Not empty when given. |
| `note` | string | no | - | Replaces the whole note (see `append_to_note`). |
| `dueDate`, `deferDate`, `plannedDate` | date | no | - | `plannedDate` requires an OmniFocus database migrated for planned dates. |
| `flagged` | boolean | no | - | |
| `tags` | array of strings | no | - | Replaces all tags; `[]` removes every tag. Every id or exact name must resolve before any field changes. |
| `estimatedMinutes` | integer | no | - | At least 0; a negative value fails before OmniFocus is contacted (`estimatedMinutes must be greater than or equal to 0.`). |

Omitting a field and passing `null` both leave it unchanged, so this tool cannot clear a date.
Returns the updated task, including effective dates and `plannedDate`.

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
`Task is not completed: <id>`. On a task that is completed only through
a completed project or containing task it fails with `Task <id> is completed through its project or
a containing task; reopen that instead: <name>`, naming that project or task. Returns
`{id, name, completed}`.

### `set_task_repetition`

Sets or removes a task's repetition.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `task_id` | string | yes | - | |
| `rule_string` | string or `null` | yes | - | Required; `null` clears. An iCalendar recurrence rule such as `FREQ=WEEKLY;INTERVAL=1` sets the repetition; `null` removes it. Omitting the key fails with an `isError` result (not `invalid_params`, so the model sees it), `rule_string is required: pass a repetition rule to set, or null to clear the repetition.` and changes nothing, so a call that only names a `schedule_type` cannot remove a repetition by accident. |
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
| `parent_task_id` | string | no | - | As in `move_task`. Must not be one of `task_ids` or below one of them. |

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

Returns `id`, `kind`, `absoluteFireDate`, `relativeFireOffset`, `nextFireDate` and `isSnoozed` for
each notification. `kind` is `absolute` (only `absoluteFireDate` is set), `relative` (fires relative
to the task's due date; only `relativeFireOffset` is set) or `unknown` (any other kind, such as
a notification OmniFocus reports in an invalid state; both are `null`).

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

A project is **stalled** when it is active and has at least one remaining task, then:

- for a single-action list, none of its remaining tasks is available now;
- for every other project, OmniFocus reports no `nextTask`.

Completed and dropped tasks are not remaining. Availability also excludes blocked tasks, tasks
deferred into the future, tasks with an on-hold tag, and tasks outside an active project or inbox.

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

Returns projects with `id`, `name`, `status`, `folderName`, `taskCount`, `remainingTaskCount`,
`deferDate`, `dueDate`, `completionDate`, `note`, `sequential`, `isStalled`, `nextTaskId`,
`nextTaskName`, `reviewInterval`. `taskCount` excludes OmniFocus's synthetic project root task.
`remainingTaskCount` also excludes completed and dropped tasks, including state inherited from a
containing task or project.
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

Returns the `list_projects` fields plus `completedTaskCount`, `availableTaskCount`, `modified` (the
last-modified date) and `rootTasks` (the project's top-level tasks). `completedTaskCount` includes
tasks completed directly, through a containing task, or through the completed project; dropped
tasks are excluded. `availableTaskCount` uses the same actionable-now definition as
`status: "available"`. All counts exclude OmniFocus's synthetic project root task.

### `create_project`

Creates one project.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `name` | string | yes | - | Not empty. |
| `folder` | string | no | top level | Folder id or exact name. |
| `note` | string | no | - | |
| `dueDate`, `deferDate` | date | no | - | |
| `sequential` | boolean | no | - | |

Returns the created project, with the same fields as `get_project`: `id`, `name`, `status`,
`folderName`, `taskCount` (0), `remainingTaskCount` (0), `completedTaskCount` (0),
`availableTaskCount` (0), `deferDate`, `dueDate`, `completionDate` (`null`), `modified`, `note`,
`sequential`, `isStalled` (`false`), `nextTaskId` (`null`), `nextTaskName` (`null`),
`reviewInterval` (`null`) and `rootTasks` (empty).

### `update_project`

Changes only the fields you pass.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `project_id_or_name` | string | yes | - | |
| `name` | string | no | - | Not empty when given. |
| `note` | string | no | - | Replaces the whole note. |
| `dueDate`, `deferDate` | date | no | - | |
| `flagged` | boolean | no | - | |
| `tags` | array of strings | no | - | Replaces all tags; `[]` removes every tag. Every id or exact name must resolve before any field changes. |
| `sequential` | boolean | no | - | |
| `completedByChildren` | boolean | no | - | |
| `reviewInterval` | string | no | - | `"N unit"`, N a whole number of at least 1, unit `day(s)`, `week(s)`, `month(s)` or `year(s)` in any letter case, e.g. `"2 weeks"`. |

`reviewInterval` can only change an interval the project already has; on a project without one
the call fails with `Project has no review interval to update` and
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
`Project is not completed: <value>`. Returns `{id, name, status}`.

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
`totalTaskCount` counts every action task with the tag, excluding synthetic project root tasks.
`availableTaskCount` counts tasks available now: remaining, actionable, not deferred into the
future, without an on-hold tag, and in an active project or the inbox.

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
| `parent` | string | no | top level | Id or exact name of an existing tag. |

Returns the created tag with the same fields as a `list_tags` tag: `id`, `name`, `parent`,
`availableTaskCount` (0), `totalTaskCount` (0), and `status` (`active`).

### `update_tag`

Renames a tag or changes its status.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `tag_name_or_id` | string | yes | - | |
| `name` | string | no | - | |
| `status` | string | no | - | `active`, `on_hold`, or `dropped`. |

At least one of `name` and `status` is required. Returns `{id, name, status}`.

### `delete_tag`

Deletes a tag immediately, unassigning it from linked tasks. Use `update_tag` for non-destructive
edits, and ask the user for explicit confirmation before calling.

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
(`id`, `name`, `status`) and direct subfolders (`id`, `name`). Folder status is `active`, `dropped`
or `unknown`; an OmniFocus status the server does not recognise is never reported as active.

### `create_folder`

Creates one folder.

| Key | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `name` | string | yes | - | Not empty. |
| `parent` | string | no | top level | Folder id or exact name. |

Returns `{id, name, parentName}`. `parentName` is the parent folder's name, as in `get_folder` and
`list_folders`, even when `parent` was given as an id; it is `null` for a top-level folder.

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

Looks at remaining action tasks and returns five sections plus their full counts. Completed and
dropped tasks, tasks in completed or dropped projects, and synthetic project root tasks are
excluded. Blocked tasks and tasks in on-hold projects remain eligible for date-based sections.

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
