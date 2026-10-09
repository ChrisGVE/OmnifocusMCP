//! `list_notifications` and `add_notification` report a notification's kind
//! from `notification.kind`, and read only the fire-date field of that kind.
//!
//! The scripts used to infer the kind from `initialFireDate`, but OmniJS sets
//! `initialFireDate` for every notification: the OmniFocus interface text
//! (`OmniFocusModel-Interfaces.strings`, `OFMAlarm.initialFireDate`) says "For
//! due or defer-relative notifications, this date will change with its `task`
//! object's due and defer dates". So every due-relative notification came back
//! as `kind: "absolute"` with `relativeFireOffset: null` (audit finding CR-002).
//!
//! The same interface text says reading `absoluteFireDate` throws unless the
//! kind is absolute, and reading `relativeFireOffset` throws unless it is due-
//! or defer-relative. The fake notifications below throw the same way, so a
//! script that reads the wrong field fails here as it would in OmniFocus.
//!
//! Each script runs in JavaScriptCore against a fake task; the assertions are
//! on what the tool returns, not on its text.

mod common;

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use omnifocus_mcp::{
    jxa::JxaRunner,
    tools::tasks::{add_notification, list_notifications},
};
use serde_json::{json, Value};

/// A fake `Task.Notification.Kind` enum and a single task whose notifications
/// are built by `fakeNotification`. Enum members are objects, as in OmniJS, so
/// only a comparison against the member itself recognises a kind.
///
/// Every fake notification carries an `initialFireDate`, except an `Unknown`
/// one (a notification in an invalid state has no meaningful fire date).
/// `task.addNotification` returns an absolute notification for a `Date` and a
/// due-relative one for a number, like OmniJS.
const FAKE_TASK: &str = r#"function FakeEnum(name) { this.name = name; }
FakeEnum.prototype.toString = function () { return "[object Task.Notification.Kind: " + this.name + "]"; };
var Task = {
  Status: { Available: new FakeEnum("Available") },
  Notification: { Kind: {
    Absolute: new FakeEnum("Absolute"),
    DueRelative: new FakeEnum("DueRelative"),
    Unknown: new FakeEnum("Unknown")
  } },
  byIdentifier: function (id) {
    return document.flattenedTasks.find(item => item.id.primaryKey === id) || null;
  }
};
var Project = { Status: { Active: new FakeEnum("Active") } };
var FIRE = new Date(Date.UTC(2026, 9, 9, 7, 0));
var NEXT = new Date(Date.UTC(2026, 9, 9, 7, 0));
function fakeNotification(id, kind, payload) {
  const Kind = Task.Notification.Kind;
  const notification = { id: { primaryKey: id }, kind: kind, nextFireDate: NEXT,
    isSnoozed: false, initialFireDate: kind === Kind.Unknown ? null : FIRE };
  Object.defineProperty(notification, "absoluteFireDate", { get: function () {
    if (kind !== Kind.Absolute) throw new Error("absoluteFireDate read on a " + kind.name + " notification");
    return payload;
  } });
  Object.defineProperty(notification, "relativeFireOffset", { get: function () {
    if (kind !== Kind.DueRelative) throw new Error("relativeFireOffset read on a " + kind.name + " notification");
    return payload;
  } });
  return notification;
}
var task = { id: { primaryKey: "t1" }, project: null, effectiveDueDate: FIRE,
  notifications: [],
  addNotification: function (fireAt) {
    const Kind = Task.Notification.Kind;
    const created = fireAt instanceof Date
      ? fakeNotification("added", Kind.Absolute, fireAt)
      : fakeNotification("added", Kind.DueRelative, fireAt);
    this.notifications.push(created);
    return created;
  } };
var document = { flattenedTasks: [task] };"#;

/// Records the script it is asked to run and returns an empty list.
struct CapturingRunner {
    scripts: Arc<Mutex<Vec<String>>>,
}

impl CapturingRunner {
    fn new() -> Self {
        Self {
            scripts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn last_script(&self) -> String {
        self.scripts
            .lock()
            .expect("script lock")
            .last()
            .cloned()
            .expect("a script was run")
    }
}

impl JxaRunner for CapturingRunner {
    fn run_omnijs<'a>(
        &'a self,
        script: &'a str,
    ) -> Pin<Box<dyn Future<Output = omnifocus_mcp::error::Result<Value>> + Send + 'a>> {
        self.scripts
            .lock()
            .expect("script lock")
            .push(script.to_string());
        Box::pin(async move { Ok(json!([])) })
    }
}

/// Runs `script` against the fake task after `setup` (JavaScript that fills
/// `task.notifications`) and returns what the script returned.
fn run_against_task(tool: &str, script: &str, setup: &str) -> Value {
    common::assert_script_compiles(tool, script);
    let prelude = format!("{FAKE_TASK}\n{setup}");
    let output = common::run_jsc(
        &prelude,
        &format!(
            "try {{ const result = (function () {{\n{script}\n}})(); print(JSON.stringify(result)); }}\n\
             catch (error) {{ print(\"ERROR: \" + error.message); }}"
        ),
    );
    serde_json::from_str(&output).unwrap_or_else(|_| panic!("{tool} returned {output}"))
}

/// Lists the notifications of a task holding the one built by `notification`.
async fn listed(notification: &str) -> Value {
    let runner = CapturingRunner::new();
    list_notifications(&runner, "t1")
        .await
        .expect("script builds");
    let setup = format!("task.notifications.push({notification});");
    let listed = run_against_task("list_notifications", &runner.last_script(), &setup);
    assert_eq!(listed.as_array().map(Vec::len), Some(1), "{listed}");
    listed[0].clone()
}

#[tokio::test]
async fn list_reports_an_absolute_notification_with_its_fire_date() {
    let notification =
        listed(r#"fakeNotification("n1", Task.Notification.Kind.Absolute, FIRE)"#).await;
    assert_eq!(notification["kind"], "absolute");
    assert_eq!(notification["absoluteFireDate"], "2026-10-09T07:00:00.000Z");
    assert_eq!(notification["relativeFireOffset"], Value::Null);
    assert_eq!(notification["nextFireDate"], "2026-10-09T07:00:00.000Z");
    assert_eq!(notification["isSnoozed"], false);
}

#[tokio::test]
async fn list_reports_a_due_relative_notification_as_relative_with_its_offset() {
    let notification =
        listed(r#"fakeNotification("n2", Task.Notification.Kind.DueRelative, -60)"#).await;
    assert_eq!(notification["kind"], "relative");
    assert_eq!(notification["absoluteFireDate"], Value::Null);
    assert_eq!(notification["relativeFireOffset"], -60);
}

#[tokio::test]
async fn list_reports_an_unknown_notification_without_reading_either_fire_field() {
    let notification =
        listed(r#"fakeNotification("n3", Task.Notification.Kind.Unknown, null)"#).await;
    assert_eq!(notification["kind"], "unknown");
    assert_eq!(notification["absoluteFireDate"], Value::Null);
    assert_eq!(notification["relativeFireOffset"], Value::Null);
}

#[tokio::test]
async fn add_reports_an_absolute_notification_as_absolute() {
    let runner = CapturingRunner::new();
    add_notification(&runner, "t1", Some("2026-10-09T07:00:00Z"), None)
        .await
        .expect("script builds");
    let added = run_against_task("add_notification", &runner.last_script(), "");
    assert_eq!(added["kind"], "absolute");
    assert_eq!(added["absoluteFireDate"], "2026-10-09T07:00:00.000Z");
    assert_eq!(added["relativeFireOffset"], Value::Null);
}

#[tokio::test]
async fn add_reports_a_due_relative_notification_as_relative_with_its_offset() {
    let runner = CapturingRunner::new();
    add_notification(&runner, "t1", None, Some(-3600.0))
        .await
        .expect("script builds");
    let added = run_against_task("add_notification", &runner.last_script(), "");
    assert_eq!(added["kind"], "relative");
    assert_eq!(added["absoluteFireDate"], Value::Null);
    assert_eq!(added["relativeFireOffset"], -3600);
}
