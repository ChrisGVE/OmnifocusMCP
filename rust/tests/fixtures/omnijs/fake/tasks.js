// The strict OmniJS fake: `Task` objects.

(function installTasks(F) {
  "use strict";
  const { INTERNAL, record, hosts, hostArray, location, getters, accessor } = F;

  F.createTaskRecord = (fields) => {
    const task = {
      kind: "task",
      id: fields.id || F.nextId("task"),
      name: fields.name,
      note: fields.note || "",
      flagged: !!fields.flagged,
      dueDate: fields.dueDate || null,
      deferDate: fields.deferDate || null,
      plannedDate: fields.plannedDate || null,
      completionDate: fields.completionDate || null,
      dropDate: fields.dropDate || null,
      estimatedMinutes:
        fields.estimatedMinutes === undefined ? null : fields.estimatedMinutes,
      sequential: !!fields.sequential,
      completedByChildren: !!fields.completedByChildren,
      shouldUseFloatingTimeZone: false,
      added: fields.added || F.now(),
      modified: fields.modified || F.now(),
      repetitionRule: fields.repetitionRule || null,
      tags: [],
      children: [],
      notifications: [],
      parent: null,
      projectOf: null, // the project whose root task this is
      inboxTop: false,
    };
    task.host = taskHost(task);
    F.db.tasks.push(task);
    return task;
  };

  F.tagRecords = (list) =>
    Array.from(list).map((host) => {
      const tag = record(host);
      if (!tag || tag.kind !== "tag")
        throw new TypeError("OmniJS strict fake: not a Tag: " + host);
      return tag;
    });

  // "If the task is not completed, marks it as complete with the given
  // completion date (or the current date if no date is specified)."
  F.markTaskComplete = (task, date) => {
    if (task.repetitionRule) {
      const occurrence = F.createTaskRecord({
        name: task.name,
        note: task.note,
        flagged: task.flagged,
        dueDate: task.dueDate,
        deferDate: task.deferDate,
        plannedDate: task.plannedDate,
        estimatedMinutes: task.estimatedMinutes,
        repetitionRule: null,
        completionDate: date || F.now(),
      });
      occurrence.tags = task.tags.slice();
      return occurrence;
    }
    if (!task.completionDate) {
      task.completionDate = date || F.now();
      task.modified = F.now();
    }
    return task;
  };

  function taskFields(raw, task) {
    F.fieldAccessors(raw, task, [
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
      "added",
      "modified",
      "completionDate",
      "dropDate",
    ]);
    accessor(
      raw,
      "repetitionRule",
      () => task.repetitionRule,
      (value) => {
        if (
          value !== null &&
          !(record(value) && record(value).kind === "repetitionRule")
        ) {
          throw new TypeError(
            "repetitionRule must be a Task.RepetitionRule or null",
          );
        }
        task.repetitionRule = value;
      },
    );
  }

  function taskComputed(raw, task) {
    getters(raw, {
      id: () => F.identifier(task.id, "Task"),
      url: () => F.urlFor("task", task.id),
      active: () => !task.dropDate,
      effectiveActive: () => !F.effectiveDropDate(task),
      completed: () => !!task.completionDate,
      effectiveCompletionDate: () => F.effectiveCompletionDate(task),
      effectiveCompletedDate: () => F.effectiveCompletionDate(task),
      effectiveDropDate: () => F.effectiveDropDate(task),
      effectiveDueDate: () => F.effectiveDueDate(task),
      effectiveDeferDate: () => F.effectiveDeferDate(task),
      effectivePlannedDate: () => F.effectivePlannedDate(task),
      effectiveFlagged: () =>
        F.selfAndAncestors(task).some((item) => item.flagged),
      taskStatus: () => F.taskStatus(task),
      noteText: () => task.note,
    });
  }

  function taskRelations(raw, task) {
    const children = () => hostArray("TaskArray", hosts(task.children));
    const descendants = () =>
      hostArray("TaskArray", hosts(F.descendants(task)));
    getters(raw, {
      hasChildren: () => task.children.length > 0,
      children,
      tasks: children,
      flattenedChildren: descendants,
      flattenedTasks: descendants,
      parent: () => (task.parent ? task.parent.host : null),
      project: () => (task.projectOf ? task.projectOf.host : null),
      containingProject: () => {
        const project = F.containingProject(task);
        return project ? project.host : null;
      },
      inInbox: () => task.inboxTop,
      assignedContainer: () => null,
      tags: () => hostArray("TagArray", hosts(task.tags)),
      notifications: () => hostArray("Array", hosts(task.notifications)),
      linkedFileURLs: () => hostArray("Array", []),
      attachments: () => hostArray("Array", []),
      beginning: () => location("beginning", task),
      ending: () => location("ending", task),
      before: () => location("before", task.parent || "inbox", task),
      after: () => location("after", task.parent || "inbox", task),
    });
  }

  function taskMethods(raw, task) {
    const addTags = (tags) =>
      F.tagRecords(tags).forEach((tag) => {
        if (!task.tags.includes(tag)) task.tags.push(tag);
      });
    const removeTags = (tags) => {
      const removed = F.tagRecords(tags);
      task.tags = task.tags.filter((tag) => !removed.includes(tag));
    };
    Object.assign(raw, {
      markComplete: (date) => F.markTaskComplete(task, date).host,
      markIncomplete: () => {
        task.completionDate = null;
        task.modified = F.now();
      },
      drop: (_allOccurrences, date) => {
        task.dropDate = date || F.now();
      },
      addTag: (tag) => addTags([tag]),
      addTags,
      removeTag: (tag) => removeTags([tag]),
      removeTags,
      clearTags: () => {
        task.tags = [];
      },
      appendStringToNote: (text) => {
        task.note += text;
      },
      addNotification: (info) => F.addNotificationTo(task, info),
      removeNotification: (notification) =>
        F.removeNotificationFrom(task, notification),
      childNamed: (name) => F.findNamed(task.children, name),
      taskNamed: (name) => F.findNamed(task.children, name),
    });
  }

  function taskHost(task) {
    const raw = Object.create(F.TaskConstructor.prototype);
    raw[INTERNAL] = task;
    taskFields(raw, task);
    taskComputed(raw, task);
    taskRelations(raw, task);
    taskMethods(raw, task);
    return F.hostObject("Task", raw);
  }
})(globalThis.__omnijsFake);
