// The strict OmniJS fake: what a task's derived members answer (effective
// dates, `taskStatus`) and how tasks are placed in the tree.

(function installTaskRules(F) {
  "use strict";
  const { Date, FIXED_NOW, TaskStatus, ProjectStatus, TagStatus, db } = F;
  const DAY_MS = 24 * 60 * 60 * 1000;

  F.ancestors = (task) => {
    const chain = [];
    for (let current = task.parent; current; current = current.parent)
      chain.push(current);
    return chain;
  };
  const selfAndAncestors = (task) => [task, ...F.ancestors(task)];
  F.selfAndAncestors = selfAndAncestors;
  F.containingProject = (task) => {
    let root = task;
    while (root.parent) root = root.parent;
    return root.projectOf || null;
  };
  F.descendants = (task) =>
    task.children.flatMap((child) => [child, ...F.descendants(child)]);

  const firstSet = (task, field) => {
    const found = selfAndAncestors(task).find((item) => item[field]);
    return found ? found[field] : null;
  };
  const extreme = (task, field, pick) => {
    const times = selfAndAncestors(task)
      .filter((item) => item[field])
      .map((item) => item[field].getTime());
    return times.length ? new Date(pick(...times)) : null;
  };
  // A project's `Done` status completes its tasks too ("a task may be
  // effectively considered completed if a containing task is marked
  // completed").
  F.effectiveCompletionDate = (task) => {
    const own = firstSet(task, "completionDate");
    if (own) return own;
    const project = F.containingProject(task);
    return project && project.status === ProjectStatus.Done
      ? project.root.completionDate
      : null;
  };
  F.effectiveDropDate = (task) => firstSet(task, "dropDate");
  F.effectivePlannedDate = (task) => firstSet(task, "plannedDate");
  F.effectiveDueDate = (task) => extreme(task, "dueDate", Math.min);
  F.effectiveDeferDate = (task) => extreme(task, "deferDate", Math.max);
  F.isRemaining = (task) =>
    !F.effectiveCompletionDate(task) && !F.effectiveDropDate(task);

  function isBlocked(task) {
    const project = F.containingProject(task);
    const defer = F.effectiveDeferDate(task);
    if (defer && defer.getTime() > FIXED_NOW) return true;
    if (project && project.status === ProjectStatus.OnHold) return true;
    if (task.tags.some((tag) => tag.status === TagStatus.OnHold)) return true;
    if (!task.parent || !task.parent.sequential) return false;
    const siblings = task.parent.children;
    return siblings.slice(0, siblings.indexOf(task)).some(F.isRemaining);
  }

  // `Task.Status`, in the plainest reading of the reference: dropped and
  // completed first, then the documented blocking causes (a future defer
  // date, an on-hold project or tag, an earlier remaining sibling in a
  // sequential container), then overdue and due within two days.
  F.taskStatus = (task) => {
    if (F.effectiveDropDate(task)) return TaskStatus.Dropped;
    if (F.effectiveCompletionDate(task)) return TaskStatus.Completed;
    if (isBlocked(task)) return TaskStatus.Blocked;
    const due = F.effectiveDueDate(task);
    if (due && due.getTime() < FIXED_NOW) return TaskStatus.Overdue;
    if (due && due.getTime() - FIXED_NOW <= 2 * DAY_MS)
      return TaskStatus.DueSoon;
    return TaskStatus.Available;
  };

  // Where a task goes, from a location or, as `new Task` and `moveTasks`
  // accept, a Project or a Task; nothing means the end of the inbox.
  F.taskDestination = (position) => {
    if (position === null || position === undefined)
      return { inbox: true, kind: "ending" };
    const target = F.record(position);
    if (target && target.location) {
      const { container, kind, sibling } = target;
      if (container === "inbox") return { inbox: true, kind, sibling };
      if (container.kind === "task")
        return { parent: container, kind, sibling };
    } else if (target && target.kind === "project") {
      return { parent: target.root, kind: "ending" };
    } else if (target && target.kind === "task") {
      return { parent: target, kind: "ending" };
    }
    throw new TypeError("OmniJS strict fake: not a task location: " + position);
  };

  F.placeTask = (task, destination) => {
    for (
      let ancestor = destination.parent;
      ancestor;
      ancestor = ancestor.parent
    ) {
      // Undocumented in OmniFocus; the server must never ask for it.
      if (ancestor === task)
        throw new Error(
          "OmniJS strict fake: a task cannot move into its own subtree",
        );
    }
    if (task.parent) F.removeFrom(task.parent.children, task);
    F.removeFrom(db.inbox, task);
    task.parent = destination.inbox ? null : destination.parent;
    task.inboxTop = !!destination.inbox;
    const list = destination.inbox ? db.inbox : destination.parent.children;
    F.insertInto(list, task, destination.kind, destination.sibling);
  };
})(globalThis.__omnijsFake);
