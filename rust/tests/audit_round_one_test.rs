//! Regression tests for audit round one: shared task status semantics, DST
//! calendar boundaries, predicate parity, and project comparator equality.

#[allow(dead_code)]
mod common;
#[path = "support/omnijs_harness.rs"]
#[allow(dead_code)]
mod omnijs_harness;

use std::collections::BTreeSet;

use omnifocus_mcp::tools::{
    forecast::get_forecast,
    projects::list_projects,
    tasks::{get_task_counts_with_added_changed, list_tasks_with_added_changed},
};

use omnifocus_mcp::jxa::JxaRunner;
use omnijs_harness::StrictOmniJs;

const ZURICH: &str = "Europe/Zurich";

#[tokio::test]
async fn inherited_dates_agree_across_list_counts_and_forecast() {
    let runner = StrictOmniJs::new("UTC");
    let listed = list_tasks_with_added_changed(
        &runner,
        Some("Inherited Dates"),
        None,
        None,
        "any",
        None,
        "overdue",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        "asc",
        20,
    )
    .await
    .expect("list_tasks should run");
    assert_eq!(
        listed
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        ["task-inherited-dates"]
    );

    let counts = get_task_counts_with_added_changed(
        &runner,
        Some("Inherited Dates"),
        None,
        None,
        "any",
        None,
        None,
        None,
        None,
        None,
        None,
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
    .expect("get_task_counts should run");
    assert_eq!(counts.total, 1);
    assert_eq!(counts.overdue, 1);
    assert_eq!(counts.deferred, 1);

    let forecast = get_forecast(&runner, 20)
        .await
        .expect("get_forecast should run");
    let ids = |key: &str| -> BTreeSet<String> {
        forecast[key]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|task| task["id"].as_str().map(str::to_string))
            .collect()
    };
    assert!(ids("overdue").contains("task-inherited-dates"));
    assert!(ids("deferred").contains("task-inherited-dates"));
    assert!(forecast["counts"]["overdueCount"].as_i64().unwrap() >= 1);
    assert!(forecast["counts"]["deferredCount"].as_i64().unwrap() >= 1);
}

#[tokio::test]
async fn list_and_counts_have_matching_sets_for_varied_filters() {
    let cases = [
        (None, None, None, None),
        (Some("Launch"), None, None, None),
        (None, Some(true), None, None),
        (None, None, Some("2026-10-11"), None),
        (None, None, None, Some("2026-10-10")),
    ];
    for (project, flagged, due_after, defer_before) in cases {
        let runner = StrictOmniJs::with_now("UTC", omnijs_harness::FIXED_NOW);
        let listed = list_tasks_with_added_changed(
            &runner,
            project,
            None,
            None,
            "any",
            flagged,
            "all",
            None,
            due_after,
            None,
            defer_before,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            "asc",
            100,
        )
        .await
        .expect("list_tasks should run");
        let counts = get_task_counts_with_added_changed(
            &runner,
            project,
            None,
            None,
            "any",
            flagged,
            None,
            due_after,
            None,
            defer_before,
            None,
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
        .expect("get_task_counts should run");
        let listed_ids: BTreeSet<String> = listed.into_iter().map(|task| task.id).collect();
        let production_script = runner
            .scripts()
            .first()
            .cloned()
            .expect("list_tasks should record its production script");
        let id_probe = production_script.replace(
            "return tasks.map(mapListedTask);",
            "return filteredTasks.map(task => task.id.primaryKey);",
        );
        let matched_ids: BTreeSet<String> = runner
            .run_omnijs(&id_probe)
            .await
            .expect("production predicate probe should run")
            .as_array()
            .expect("predicate probe should return ids")
            .iter()
            .map(|id| id.as_str().expect("task id should be a string").to_string())
            .collect();
        assert_eq!(listed_ids, matched_ids);

        let deliberately_different = production_script
            .replace(
                "if (!taskMatchesFilters(task)) return false;",
                "if (false) return false;",
            )
            .replace(
                "return tasks.map(mapListedTask);",
                "return filteredTasks.map(task => task.id.primaryKey);",
            );
        let different_ids: BTreeSet<String> = runner
            .run_omnijs(&deliberately_different)
            .await
            .expect("deliberate predicate probe should run")
            .as_array()
            .expect("deliberate probe should return ids")
            .iter()
            .map(|id| id.as_str().expect("task id should be a string").to_string())
            .collect();
        assert_ne!(listed_ids, different_ids);
        assert_eq!(
            matched_ids.len() as i32,
            counts.total,
            "filter case {project:?} {flagged:?} {due_after:?} {defer_before:?}"
        );
    }
}

#[tokio::test]
async fn dst_old_millisecond_day_reproduces_failure_before_calendar_fix() {
    let runner = StrictOmniJs::with_now(ZURICH, "2026-10-25T12:00:00");
    let forecast = get_forecast(&runner, 100)
        .await
        .expect("forecast should run");
    let due_today = forecast["dueToday"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|task| task["id"].as_str())
        .collect::<BTreeSet<_>>();
    assert!(due_today.contains("task-dst-autumn"));
}

#[tokio::test]
async fn dst_calendar_day_keeps_due_today_and_excludes_next_day() {
    let runner = StrictOmniJs::with_now(ZURICH, "2026-03-29T12:00:00");
    let forecast = get_forecast(&runner, 100)
        .await
        .expect("forecast should run");
    let due_today = forecast["dueToday"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|task| task["id"].as_str())
        .collect::<BTreeSet<_>>();
    assert!(!due_today.contains("task-dst-spring"));
}

#[tokio::test]
async fn project_both_null_sort_is_stable_in_both_directions() {
    let runner = StrictOmniJs::with_now("UTC", omnijs_harness::FIXED_NOW);
    let ids = |value: serde_json::Value| {
        value
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|project| project["id"].as_str().map(str::to_string))
            .collect::<Vec<_>>()
    };
    let ascending = ids(list_projects(
        &runner,
        None,
        "active",
        None,
        None,
        false,
        Some("dueDate"),
        "asc",
        100,
    )
    .await
    .expect("ascending list_projects should run"));
    let descending = ids(list_projects(
        &runner,
        None,
        "active",
        None,
        None,
        false,
        Some("dueDate"),
        "desc",
        100,
    )
    .await
    .expect("descending list_projects should run"));
    let relative_order = |items: &[String]| {
        (
            items
                .iter()
                .position(|id| id == "project-null-sort-a")
                .unwrap(),
            items
                .iter()
                .position(|id| id == "project-null-sort-b")
                .unwrap(),
        )
    };
    assert_eq!(relative_order(&ascending), relative_order(&descending));
}
