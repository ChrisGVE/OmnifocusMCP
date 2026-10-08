//! Task tools: the OmniJS script builders behind every tool that reads or
//! changes OmniFocus tasks.
//!
//! Each submodule holds one group of tools, and this module re-exports their
//! public items, so callers keep using `crate::tools::tasks::<name>`.
//!
//! - `list` and `search`: `list_tasks` and `search_tasks`. Both run the one
//!   script built in `listing_script` from the filters in `filters`.
//! - `counts`: `get_task_counts`, aggregate counts for the same filters.
//! - `reads`: the inbox, one task's details, and a task's subtasks.
//! - `notifications`: listing, adding and removing a task's notifications.
//! - `create`: creating one task or subtask, and duplicating a task.
//! - `update`: changing a task's fields or its repetition rule.
//! - `lifecycle`: completing and deleting one task.
//! - `moves`: moving one task.
//! - `batch`: the batch variants of create, delete and move.
//! - `inputs`: validation and normalisation shared by the tools above.

mod batch;
mod counts;
mod create;
mod filters;
mod inputs;
mod lifecycle;
mod list;
mod listing_script;
mod moves;
mod notifications;
mod reads;
mod search;
mod update;

pub use batch::{create_tasks_batch, delete_tasks_batch, move_tasks_batch};
pub use counts::{get_task_counts, get_task_counts_with_added_changed};
pub use create::{create_subtask, create_task, duplicate_task, CreateTaskInput};
pub use lifecycle::{complete_task, delete_task};
pub use list::{list_tasks, list_tasks_with_added_changed, list_tasks_with_planned};
pub use moves::move_task;
pub use notifications::{add_notification, list_notifications, remove_notification};
pub use reads::{get_inbox, get_task, list_subtasks};
pub use search::{search_tasks, search_tasks_with_added_changed, search_tasks_with_planned};
pub use update::{set_task_repetition, update_task};
