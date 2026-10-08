//! Project tools: the OmniJS script builders behind every tool that reads or
//! changes OmniFocus projects.
//!
//! Each submodule holds one group of tools, and this module re-exports their
//! public functions, so callers keep using `crate::tools::projects::<name>`.
//!
//! - `list`: `list_projects`, filtered by folder, status and completion.
//! - `reads`: searching projects, counting them, and one project's details.
//! - `writes`: creating, moving, re-statusing and updating a project.
//! - `lifecycle`: completing, reopening and deleting projects.

mod lifecycle;
mod list;
mod reads;
mod writes;

pub use lifecycle::{complete_project, delete_project, delete_projects_batch, uncomplete_project};
pub use list::list_projects;
pub use reads::{get_project, get_project_counts, search_projects};
pub use writes::{create_project, move_project, set_project_status, update_project};
