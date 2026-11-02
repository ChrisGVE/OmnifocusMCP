// The database every row of `tests/all_tools_jsc_test.rs` runs against,
// built fresh for each script with the fake's `__harness` API. Ids are fixed
// so the rows can name objects; the fake's clock reads 2026-10-09 10:00 UTC.
//
// Objects whose names start with "Scrap" exist to be deleted by the delete
// rows; nothing else refers to them.

(function buildScenario(h) {
  "use strict";

  // Folders: Work > Clients, and two top-level folders to delete.
  const work = h.folder({ id: "folder-work", name: "Work" });
  const clients = h.folder({ id: "folder-clients", name: "Clients", in: work });
  h.folder({ id: "folder-scrap", name: "Scrap Folder" });
  h.folder({ id: "folder-scrap-2", name: "Scrap Folder Two" });

  // Tags: Home, Office > Desk, an on-hold Waiting, and two to delete.
  const home = h.tag({ id: "tag-home", name: "Home" });
  const office = h.tag({ id: "tag-office", name: "Office" });
  const desk = h.tag({ id: "tag-desk", name: "Desk", in: office });
  h.tag({ id: "tag-waiting", name: "Waiting", status: Tag.Status.OnHold });
  h.tag({ id: "tag-dropped", name: "Dropped", status: Tag.Status.Dropped });
  h.tag({ id: "tag-scrap", name: "Scrap Tag" });
  h.tag({ id: "tag-scrap-2", name: "Scrap Tag Two" });

  // Projects: an active project in Work, a single-action list at the top
  // level, a completed and an on-hold project in Clients, two to delete.
  const launch = h.project({
    id: "project-launch",
    name: "Launch",
    in: work,
    note: "Ship the product",
    dueDate: h.date("2026-10-20T17:00:00Z"),
  });
  const errands = h.project({
    id: "project-errands",
    name: "Errands",
    containsSingletonActions: true,
  });
  h.project({
    id: "project-old",
    name: "Old",
    in: clients,
    status: Project.Status.Done,
    completionDate: h.date("2026-09-01T12:00:00Z"),
  });
  h.project({
    id: "project-paused",
    name: "Paused",
    in: clients,
    status: Project.Status.OnHold,
  });
  h.project({ id: "project-scrap", name: "Scrap Project" });
  h.project({ id: "project-scrap-2", name: "Scrap Project Two" });

  // Tasks in Launch: a due task with two levels of subtasks and two
  // notifications, a flagged deferred task, and a completed task.
  const draft = h.task({
    id: "task-draft",
    name: "Draft plan",
    in: launch,
    note: "first draft",
    dueDate: h.date("2026-10-10T15:00:00Z"),
    estimatedMinutes: 30,
    tags: [home],
  });
  const outline = h.task({ id: "task-outline", name: "Outline", in: draft });
  h.task({ id: "task-section", name: "Section 1", in: outline });
  h.notify(draft, h.date("2026-10-10T08:00:00Z"), "notification-absolute");
  h.notify(draft, -60, "notification-relative");
  h.task({
    id: "task-review",
    name: "Review plan",
    in: launch,
    flagged: true,
    deferDate: h.date("2026-10-12T08:00:00Z"),
    tags: [office],
  });
  h.task({
    id: "task-ship",
    name: "Ship",
    in: launch,
    completionDate: h.date("2026-10-01T12:00:00Z"),
  });

  // Tasks in Errands, one of them repeating.
  h.task({
    id: "task-milk",
    name: "Buy milk",
    in: errands,
    plannedDate: h.date("2026-10-09T09:00:00Z"),
    tags: [home, desk],
  });
  const water = h.task({
    id: "task-water",
    name: "Water plants",
    in: errands,
    repetitionRule: new Task.RepetitionRule(
      "FREQ=WEEKLY",
      null,
      Task.RepetitionScheduleType.Regularly,
    ),
  });
  h.notify(water, h.date("2026-10-11T08:00:00Z"), "notification-water");

  // The inbox: an overdue task, one with a note, three to delete.
  h.task({
    id: "task-call",
    name: "Call bank",
    dueDate: h.date("2026-10-08T09:00:00Z"),
  });
  h.task({
    id: "task-read",
    name: "Read article",
    note: "from the newsletter",
  });
  h.task({ id: "task-scrap", name: "Scrap Task" });
  h.task({ id: "task-scrap-2", name: "Scrap Task Two" });
  h.task({ id: "task-scrap-3", name: "Scrap Task Three" });
  h.task({ id: "task-dropped-inbox", name: "Dropped inbox item", dropDate: h.date("2026-10-08T12:00:00Z") });

  h.perspective({ id: "perspective-weekly", name: "Weekly Review" });
})(__harness);
