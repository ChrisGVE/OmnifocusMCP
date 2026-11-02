//! Regression coverage for the shared task/tag result contracts and inbox
//! filtering against the strict OmniJS fake.

#[allow(dead_code)]
mod common;
#[path = "support/omnijs_harness.rs"]
#[allow(dead_code)]
mod omnijs_harness;

use omnifocus_mcp::tools::{
    forecast::get_forecast,
    projects::{complete_project, get_project, set_project_status},
    tags::{list_tags, search_tags},
    tasks::{complete_task, duplicate_task, get_inbox, get_task, update_task},
};
use omnijs_harness::StrictOmniJs;
const UTC: &str = "UTC";

#[tokio::test]
async fn inbox_excludes_dropped_items() {
    let runner = StrictOmniJs::new(UTC);
    let result = get_inbox(&runner, 100).await.expect("inbox succeeds");
    assert!(!result.iter().any(|task| task.id == "task-dropped-inbox"));
}

#[tokio::test]
async fn task_status_is_present_on_detail_children_and_write_results() {
    let runner = StrictOmniJs::new(UTC);
    let detail = get_task(&runner, "task-draft")
        .await
        .expect("detail succeeds");
    assert_eq!(detail["taskStatus"], "due_soon");
    assert_eq!(detail["children"][0]["taskStatus"], "due_soon");

    let updated = update_task(
        &runner,
        "task-draft",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("update succeeds");
    assert_eq!(updated["taskStatus"], "due_soon");

    let completed = complete_task(&runner, "task-water")
        .await
        .expect("complete succeeds");
    assert_eq!(completed["completed"], true);
    assert_eq!(completed["taskStatus"], "completed");
}

#[tokio::test]
async fn shared_summaries_include_dates_and_repetition() {
    let runner = StrictOmniJs::new(UTC);
    let forecast = get_forecast(&runner, 10).await.expect("forecast succeeds");
    let overdue = &forecast["overdue"][0];
    assert!(overdue.get("addedDate").is_some());
    assert!(overdue.get("changedDate").is_some());
    assert!(overdue.get("plannedDate").is_some());

    let duplicate = duplicate_task(&runner, "task-water", false)
        .await
        .expect("duplicate succeeds");
    assert_eq!(duplicate["repetitionRule"], "FREQ=WEEKLY");
    assert!(duplicate.get("notifications").is_some());
}

#[tokio::test]
async fn project_roots_and_status_results_are_live() {
    let runner = StrictOmniJs::new(UTC);
    let project = get_project(&runner, "project-launch")
        .await
        .expect("project succeeds");
    assert_eq!(project["rootTasks"][0]["taskStatus"], "due_soon");

    let completed = complete_project(&runner, "project-launch")
        .await
        .expect("complete succeeds");
    assert_eq!(completed["status"], "completed");
    let changed = set_project_status(&runner, "project-old", "on_hold")
        .await
        .expect("status succeeds");
    assert_eq!(changed["status"], "on_hold");
}

#[tokio::test]
async fn tag_statuses_use_documented_enum_values() {
    let runner = StrictOmniJs::new(UTC);
    let listed = list_tags(&runner, "all", None, "asc", 100)
        .await
        .expect("list succeeds");
    let listed = listed.as_array().expect("tag list is an array");
    assert!(listed.iter().any(|tag| tag["status"] == "on_hold"));
    assert!(listed.iter().any(|tag| tag["status"] == "dropped"));
    let searched = search_tags(&runner, "Drop", 10)
        .await
        .expect("search succeeds");
    assert_eq!(searched[0]["status"], "dropped");
}
