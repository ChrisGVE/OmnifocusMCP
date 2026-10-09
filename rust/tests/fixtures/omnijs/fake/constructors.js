// The strict OmniJS fake: the global constructors (`Task`, `Project`,
// `Folder`, `Tag`), their statics, `Task.RepetitionRule`, and
// `Perspective`.

(function installConstructors(F) {
  "use strict";
  const { INTERNAL, db, hostConstructor, hostNamespace, hostArray } = F;
  const given = (value) => value !== null && value !== undefined;
  const byId = (list, id) => {
    const found = list.find((item) => item.id === id);
    return found ? found.host : null;
  };

  // "If the rule string is not valid, an error will be thrown." "If
  // deprecated method is provided along with updated scheduleType and
  // anchorDateKey, an error will be thrown." "The system defaults will be
  // used ... if not provided."
  function createRepetitionRule(
    ruleString,
    method,
    scheduleType,
    anchorDateKey,
    catchUpAutomatically,
  ) {
    if (
      typeof ruleString !== "string" ||
      !/(^|;)FREQ=[A-Z]+/.test(ruleString)
    ) {
      throw new Error("Invalid repetition rule string: " + ruleString);
    }
    if (given(method) && (given(scheduleType) || given(anchorDateKey))) {
      throw new Error(
        "Task.RepetitionRule: method cannot be combined with scheduleType or anchorDateKey",
      );
    }
    const raw = {
      ruleString,
      method: given(method) ? method : F.RepetitionMethod.Fixed,
      scheduleType: given(scheduleType)
        ? scheduleType
        : F.RepetitionScheduleType.Regularly,
      anchorDateKey: given(anchorDateKey)
        ? anchorDateKey
        : F.AnchorDateKey.DueDate,
      catchUpAutomatically: given(catchUpAutomatically)
        ? !!catchUpAutomatically
        : false,
    };
    raw[INTERNAL] = { kind: "repetitionRule" };
    return F.hostObject("Task.RepetitionRule", raw);
  }

  F.Task = hostConstructor(
    "Task",
    (name, position) => {
      const task = F.createTaskRecord({ name });
      F.placeTask(task, F.taskDestination(position));
      return task.host;
    },
    {
      Status: F.TaskStatus,
      Notification: hostNamespace("Task.Notification", {
        Kind: F.NotificationKind,
      }),
      RepetitionRule: hostConstructor(
        "Task.RepetitionRule",
        createRepetitionRule,
        {},
      ),
      RepetitionMethod: F.RepetitionMethod,
      RepetitionScheduleType: F.RepetitionScheduleType,
      AnchorDateKey: F.AnchorDateKey,
      byIdentifier: (id) => byId(db.tasks, id),
    },
    F.TaskConstructor,
  );

  // `new Project`, `new Folder` and `new Tag` all place the new object the
  // same way their kind is moved.
  const sectionConstructor = (path, create, place, statics, target) =>
    hostConstructor(
      path,
      (name, position) => {
        const item = create({ name });
        place(item, position);
        return item.host;
      },
      statics,
      target,
    );
  F.Project = sectionConstructor(
    "Project",
    F.createProjectRecord,
    F.placeSection,
    { Status: F.ProjectStatus, byIdentifier: (id) => byId(db.projects, id) },
    F.ProjectConstructor,
  );
  F.Folder = sectionConstructor(
    "Folder",
    F.createFolderRecord,
    F.placeSection,
    { Status: F.FolderStatus, byIdentifier: (id) => byId(db.folders, id) },
    F.FolderConstructor,
  );
  F.Tag = sectionConstructor(
    "Tag",
    F.createTagRecord,
    F.placeTag,
    {
      Status: F.TagStatus,
      byIdentifier: (id) => byId(db.tags, id),
      forecastTag: null,
    },
    F.TagConstructor,
  );

  // Built-in perspectives document only `name`; custom ones are database
  // objects with an `identifier`.
  const builtIn = F.makeEnum("Perspective.BuiltIn", (name) => {
    const raw = { name };
    raw[INTERNAL] = { kind: "perspective", name };
    return F.hostObject("Perspective.BuiltIn", raw);
  });
  F.createCustomPerspective = (fields) => {
    const perspective = {
      kind: "perspective",
      id: fields.id,
      name: fields.name,
    };
    const raw = { name: fields.name, identifier: fields.id };
    raw[INTERNAL] = perspective;
    F.getters(raw, {
      id: () => F.identifier(fields.id, "Perspective"),
      url: () => F.urlFor("perspective", fields.id),
      added: () => F.now(),
      modified: () => F.now(),
    });
    perspective.host = F.hostObject("Perspective.Custom", raw);
    db.perspectives.push(perspective);
    return perspective;
  };
  const custom = () => F.hosts(db.perspectives);
  F.Perspective = hostNamespace("Perspective", {
    BuiltIn: builtIn,
    Custom: hostNamespace("Perspective.Custom", {
      get all() {
        return hostArray("Array", custom());
      },
      byIdentifier: (id) => byId(db.perspectives, id),
      byName: (name) => F.findNamed(db.perspectives, name),
    }),
    get all() {
      return hostArray("Array", [...builtIn.all, ...custom()]);
    },
    get favorites() {
      return hostArray("Array", []);
    },
  });
})(globalThis.__omnijsFake);
