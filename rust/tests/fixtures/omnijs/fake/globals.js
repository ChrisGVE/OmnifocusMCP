// The strict OmniJS fake, last part: the database's global members
// (`flattenedTasks`, `inbox`, `deleteObject`, `moveTasks`, `document`, ...),
// installed as globals because the OmniJS global object IS the database;
// then `__harness`, the scenario's API; then the shared namespace goes away.

(function installGlobals(F, global) {
  "use strict";
  const { db, hosts, hostArray, record, location } = F;

  function remove(item) {
    if (item.kind === "task") {
      [item, ...F.descendants(item)].forEach((task) =>
        F.removeFrom(db.tasks, task),
      );
      if (item.parent) F.removeFrom(item.parent.children, item);
      F.removeFrom(db.inbox, item);
    } else if (item.kind === "project") {
      remove(item.root);
      F.removeFrom(db.projects, item);
      F.removeFrom(F.sectionsOf(item.parentFolder), item);
    } else if (item.kind === "folder") {
      item.sections.slice().forEach(remove);
      F.removeFrom(db.folders, item);
      F.removeFrom(F.sectionsOf(item.parent), item);
    } else if (item.kind === "tag") {
      item.children.slice().forEach(remove);
      db.tasks.forEach((task) => F.removeFrom(task.tags, item));
      F.removeFrom(db.tags, item);
      F.removeFrom(F.tagsUnder(item.parent), item);
    } else {
      throw new TypeError(
        "OmniJS strict fake: deleteObject cannot delete a " + item.kind,
      );
    }
  }

  // "Tasks are sorted by their order in the database": projects in library
  // order, each root task before its tasks, then the inbox.
  function tasksInOrder() {
    const projects = F.flattenSections(db.library).filter(
      (item) => item.kind === "project",
    );
    const roots = [...projects.map((project) => project.root), ...db.inbox];
    return roots.flatMap((task) => [task, ...F.descendants(task)]);
  }

  // What `duplicateTasks` copies; the copy gets a new id and new dates of
  // its own.
  const COPIED_FIELDS = [
    "name",
    "note",
    "flagged",
    "dueDate",
    "deferDate",
    "plannedDate",
    "estimatedMinutes",
    "sequential",
    "completedByChildren",
    "completionDate",
    "repetitionRule",
  ];
  function copyTask(source, destination) {
    const fields = {};
    COPIED_FIELDS.forEach((name) => {
      fields[name] = source[name];
    });
    const copy = F.createTaskRecord(fields);
    copy.tags = source.tags.slice();
    F.placeTask(copy, destination);
    source.children.forEach((child) =>
      copyTask(child, { parent: copy, kind: "ending" }),
    );
    return copy;
  }

  function records(list, kind) {
    return Array.from(list).map((host) => {
      const item = record(host);
      if (!item || item.kind !== kind)
        throw new TypeError(
          "OmniJS strict fake: expected " + kind + " objects",
        );
      return item;
    });
  }

  const containerEnds = (container) => ({
    get beginning() {
      return location("beginning", container);
    },
    get ending() {
      return location("ending", container);
    },
  });
  const contains = (name, query) =>
    name.toLowerCase().includes(String(query).toLowerCase());
  const sections = (kind) =>
    F.flattenSections(db.library).filter((item) => item.kind === kind);

  // The production wrapper (`crate::jxa`) feature-tests these four names on
  // `document` and copies the documented globals onto it when absent. They
  // are not documented `Document` members; the harness allows exactly these,
  // so every script sees `document` as it would in OmniFocus.
  const documentMembers = Object.assign({}, F.documentedMembers("Document"));
  [
    "flattenedTasks",
    "flattenedProjects",
    "flattenedFolders",
    "flattenedTags",
  ].forEach((name) => {
    documentMembers[name] = "probe";
  });
  const value = (table, key) => (F.own(table, key) ? table[key] : null);

  const DATABASE = {
    Task: F.Task,
    Project: F.Project,
    Folder: F.Folder,
    Tag: F.Tag,
    Perspective: F.Perspective,
    document: F.hostObject("Document", { name: "OmniFocus" }, documentMembers),
    settings: F.hostObject("Settings", {
      objectForKey: (key) => value(F.SETTINGS, key),
      defaultObjectForKey: (key) => value(F.SETTINGS, key),
      hasNonDefaultObjectForKey: () => false,
    }),
    get flattenedTasks() {
      return hostArray("TaskArray", hosts(tasksInOrder()));
    },
    get flattenedProjects() {
      return hostArray("ProjectArray", hosts(sections("project")));
    },
    get flattenedFolders() {
      return hostArray("FolderArray", hosts(sections("folder")));
    },
    get flattenedSections() {
      return hostArray("SectionArray", hosts(F.flattenSections(db.library)));
    },
    get flattenedTags() {
      return hostArray("TagArray", hosts(F.flattenTags(db.topTags)));
    },
    get folders() {
      return hostArray(
        "FolderArray",
        hosts(db.library.filter((item) => item.kind === "folder")),
      );
    },
    get projects() {
      return hostArray(
        "ProjectArray",
        hosts(db.library.filter((item) => item.kind === "project")),
      );
    },
    get inbox() {
      return hostArray("Inbox", hosts(db.inbox), containerEnds("inbox"));
    },
    get library() {
      return hostArray("Library", hosts(db.library), containerEnds("library"));
    },
    get tags() {
      return hostArray("Tags", hosts(db.topTags), containerEnds("tags"));
    },
    deleteObject: (host) => {
      if (!record(host))
        throw new TypeError(
          "OmniJS strict fake: deleteObject expects a database object",
        );
      remove(record(host));
    },
    moveTasks: (tasks, position) => {
      const destination = F.taskDestination(position);
      records(tasks, "task").forEach((task) => F.placeTask(task, destination));
    },
    duplicateTasks: (tasks, position) => {
      const destination = F.taskDestination(position);
      return hostArray(
        "TaskArray",
        hosts(
          records(tasks, "task").map((task) => copyTask(task, destination)),
        ),
      );
    },
    moveSections: (items, position) => {
      Array.from(items).forEach((host) => {
        const item = record(host);
        if (!item || (item.kind !== "project" && item.kind !== "folder")) {
          throw new TypeError(
            "OmniJS strict fake: moveSections expects projects or folders",
          );
        }
        F.placeSection(item, position);
      });
    },
    moveTags: (tags, position) =>
      records(tags, "tag").forEach((tag) => F.placeTag(tag, position)),
    projectsMatching: (query) =>
      hostArray(
        "ProjectArray",
        hosts(db.projects.filter((p) => contains(p.name, query))),
      ),
    foldersMatching: (query) =>
      hostArray(
        "FolderArray",
        hosts(db.folders.filter((f) => contains(f.name, query))),
      ),
    tagsMatching: (query) =>
      hostArray(
        "TagArray",
        hosts(db.tags.filter((t) => contains(t.name, query))),
      ),
    projectNamed: (name) =>
      F.findNamed(
        db.library.filter((item) => item.kind === "project"),
        name,
      ),
    folderNamed: (name) =>
      F.findNamed(
        db.library.filter((item) => item.kind === "folder"),
        name,
      ),
    tagNamed: (name) => F.findNamed(db.topTags, name),
    taskNamed: (name) => F.findNamed(db.inbox, name),
    save: () => {},
    cleanUp: () => {},
  };

  const documentedGlobals = F.documentedMembers("Database");
  const constructors = ["Task", "Project", "Folder", "Tag", "Perspective"];
  Object.keys(DATABASE).forEach((name) => {
    if (!constructors.includes(name) && !F.own(documentedGlobals, name)) {
      throw new Error(
        "harness: " + name + " is not a documented Database member",
      );
    }
    const descriptor = Object.getOwnPropertyDescriptor(DATABASE, name);
    Object.defineProperty(
      global,
      name,
      Object.assign(descriptor, { configurable: false }),
    );
  });

  // The scenario's API (`scenario.js`), and the logs the harness reads.
  // Scripts under test never use it.
  const parentTask = (fields) =>
    fields.in && fields.in.kind === "project" ? fields.in.root : fields.in;
  const placed = (item, place, fields) => {
    place(item, fields.in ? fields.in.host : null);
    return item;
  };
  const harness = {
    violations: F.violations,
    gaps: F.gaps,
    auditFixtures: F.AUDIT_FIXTURES,
    date: (text) => {
      const date = new F.Date(text);
      if (Number.isNaN(date.getTime()))
        throw new Error("harness: invalid date " + text);
      return date;
    },
    folder: (fields) =>
      placed(F.createFolderRecord(fields), F.placeSection, fields),
    project: (fields) =>
      placed(F.createProjectRecord(fields), F.placeSection, fields),
    tag: (fields) => placed(F.createTagRecord(fields), F.placeTag, fields),
    task: (fields) => {
      const task = F.createTaskRecord(fields);
      const parent = parentTask(fields);
      F.placeTask(
        task,
        parent ? { parent, kind: "ending" } : { inbox: true, kind: "ending" },
      );
      (fields.tags || []).forEach((tag) => task.tags.push(tag));
      return task;
    },
    notify: (task, info, id) => record(F.addNotificationTo(task, info, id)),
    perspective: (fields) => F.createCustomPerspective(fields),
  };
  Object.defineProperty(global, "__harness", { value: Object.freeze(harness) });
  delete global.__omnijsFake;
})(globalThis.__omnijsFake, globalThis);
