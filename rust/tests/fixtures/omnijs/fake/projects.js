// The strict OmniJS fake: `Project` objects. A project's root task "holds
// the bulk of the project information" (reference, `Project.task`), so most
// members read and write the root task, which shares the project's id.

(function installProjects(F) {
  "use strict";
  const {
    INTERNAL,
    ProjectStatus,
    TaskStatus,
    record,
    hosts,
    hostArray,
    location,
    getters,
    accessor,
  } = F;
  const STATUSES = [
    ProjectStatus.Active,
    ProjectStatus.Done,
    ProjectStatus.Dropped,
    ProjectStatus.OnHold,
  ];

  F.createProjectRecord = (fields) => {
    const project = {
      kind: "project",
      status: fields.status || ProjectStatus.Active,
      containsSingletonActions: !!fields.containsSingletonActions,
      parentFolder: null,
      reviewInterval:
        fields.reviewInterval === undefined
          ? { steps: 1, unit: "weeks" }
          : fields.reviewInterval,
      lastReviewDate: fields.lastReviewDate || null,
      nextReviewDate: fields.nextReviewDate || null,
    };
    project.root = F.createTaskRecord(
      Object.assign({}, fields, { id: fields.id || F.nextId("project") }),
    );
    project.root.projectOf = project;
    project.id = project.root.id;
    Object.defineProperty(project, "name", { get: () => project.root.name });
    project.host = projectHost(project);
    F.db.projects.push(project);
    return project;
  };

  // `Project.ReviewInterval` is a value object: reading returns a copy, and
  // only that type is accepted back.
  function reviewIntervalHost(value) {
    const raw = { steps: value.steps, unit: value.unit };
    raw[INTERNAL] = { kind: "reviewInterval" };
    return F.hostObject("Project.ReviewInterval", raw);
  }

  function setStatus(project, status) {
    if (!STATUSES.includes(status))
      throw new TypeError("status must be a Project.Status");
    const root = project.root;
    project.status = status;
    if (status !== ProjectStatus.Done) root.completionDate = null;
    else if (!root.completionDate) root.completionDate = F.now();
    if (status !== ProjectStatus.Dropped) root.dropDate = null;
    else if (!root.dropDate) root.dropDate = F.now();
  }

  // "Returns the very next task that can be completed in the project, or
  // null if there is none or the project contains singleton actions."
  function nextTask(project) {
    if (project.containsSingletonActions) return null;
    const actionable = [
      TaskStatus.Available,
      TaskStatus.Next,
      TaskStatus.DueSoon,
      TaskStatus.Overdue,
    ];
    const next = F.descendants(project.root).find(
      (task) =>
        task.children.length === 0 && actionable.includes(F.taskStatus(task)),
    );
    return next ? next.host : null;
  }

  // `project.reviewInterval`: a copy on read, and only a
  // `Project.ReviewInterval` accepted on write.
  function reviewIntervalMember(raw, project) {
    accessor(
      raw,
      "reviewInterval",
      () =>
        project.reviewInterval
          ? reviewIntervalHost(project.reviewInterval)
          : null,
      (value) => {
        if (!(record(value) && record(value).kind === "reviewInterval")) {
          throw new TypeError(
            "reviewInterval must be a Project.ReviewInterval",
          );
        }
        project.reviewInterval = { steps: value.steps, unit: value.unit };
      },
    );
  }

  // The members a project reads and writes on its root task.
  const ROOT_FIELDS = [
    "name",
    "note",
    "flagged",
    "dueDate",
    "deferDate",
    "plannedDate",
    "estimatedMinutes",
    "sequential",
    "completedByChildren",
    "shouldUseFloatingTimeZone",
    "completionDate",
    "dropDate",
  ];

  function projectFields(raw, project) {
    const root = project.root;
    F.fieldAccessors(raw, root, ROOT_FIELDS);
    F.fieldAccessors(raw, project, [
      "containsSingletonActions",
      "lastReviewDate",
      "nextReviewDate",
    ]);
    accessor(
      raw,
      "status",
      () => project.status,
      (value) => setStatus(project, value),
    );
    accessor(
      raw,
      "repetitionRule",
      () => root.repetitionRule,
      (value) => {
        root.host.repetitionRule = value;
      },
    );
    reviewIntervalMember(raw, project);
  }

  function projectComputed(raw, project) {
    const root = project.root;
    getters(raw, {
      id: () => F.identifier(project.id, "Project"),
      url: () => F.urlFor("project", project.id),
      completed: () => project.status === ProjectStatus.Done,
      taskStatus: () => F.taskStatus(root),
      effectiveCompletedDate: () => F.effectiveCompletionDate(root),
      effectiveDropDate: () => F.effectiveDropDate(root),
      effectiveDueDate: () => F.effectiveDueDate(root),
      effectiveDeferDate: () => F.effectiveDeferDate(root),
      effectivePlannedDate: () => F.effectivePlannedDate(root),
      effectiveFlagged: () => root.flagged,
      noteText: () => root.note,
      defaultSingletonActionHolder: () => project.containsSingletonActions,
    });
  }

  function projectRelations(raw, project) {
    const root = project.root;
    const children = () => hostArray("TaskArray", hosts(root.children));
    const descendants = () =>
      hostArray("TaskArray", hosts(F.descendants(root)));
    getters(raw, {
      task: () => root.host,
      nextTask: () => nextTask(project),
      hasChildren: () => root.children.length > 0,
      children,
      tasks: children,
      flattenedChildren: descendants,
      flattenedTasks: descendants,
      parentFolder: () =>
        project.parentFolder ? project.parentFolder.host : null,
      tags: () => hostArray("TagArray", hosts(root.tags)),
      notifications: () => hostArray("Array", hosts(root.notifications)),
      linkedFileURLs: () => hostArray("Array", []),
      attachments: () => hostArray("Array", []),
      beginning: () => location("beginning", root),
      ending: () => location("ending", root),
      before: () =>
        location("before", project.parentFolder || "library", project),
      after: () =>
        location("after", project.parentFolder || "library", project),
    });
  }

  function projectMethods(raw, project) {
    const root = project.root;
    Object.assign(raw, {
      // "In either case, the project that has been marked completed is
      // returned."
      markComplete: (date) => {
        project.status = ProjectStatus.Done;
        F.markTaskComplete(root, date);
        return project.host;
      },
      markIncomplete: () => {
        project.status = ProjectStatus.Active;
        root.completionDate = null;
      },
      addTag: (tag) => root.host.addTag(tag),
      addTags: (tags) => root.host.addTags(tags),
      removeTag: (tag) => root.host.removeTag(tag),
      removeTags: (tags) => root.host.removeTags(tags),
      clearTags: () => root.host.clearTags(),
      appendStringToNote: (text) => root.host.appendStringToNote(text),
      addNotification: (info) => F.addNotificationTo(root, info),
      removeNotification: (notification) =>
        F.removeNotificationFrom(root, notification),
      taskNamed: (name) => F.findNamed(root.children, name),
    });
  }

  function projectHost(project) {
    const raw = Object.create(F.ProjectConstructor.prototype);
    raw[INTERNAL] = project;
    projectFields(raw, project);
    projectComputed(raw, project);
    projectRelations(raw, project);
    projectMethods(raw, project);
    return F.hostObject("Project", raw);
  }
})(globalThis.__omnijsFake);
