// The strict OmniJS fake: the enums, the in-memory database, and the small
// pieces every object kind uses (identifiers, locations, accessors).

(function installStore(F) {
  "use strict";
  const { hostObject, INTERNAL } = F;

  F.TaskStatus = F.makeEnum("Task.Status");
  F.ProjectStatus = F.makeEnum("Project.Status");
  F.TagStatus = F.makeEnum("Tag.Status");
  F.FolderStatus = F.makeEnum("Folder.Status");
  F.NotificationKind = F.makeEnum("Task.Notification.Kind");
  F.RepetitionMethod = F.makeEnum("Task.RepetitionMethod");
  F.RepetitionScheduleType = F.makeEnum("Task.RepetitionScheduleType");
  F.AnchorDateKey = F.makeEnum("Task.AnchorDateKey");

  // Records are plain objects holding the state; each has a `host`, the
  // strict Proxy scripts see, created once so identity comparisons work.
  F.db = {
    tasks: [], // every task record, project root tasks included
    projects: [],
    folders: [],
    tags: [],
    perspectives: [],
    inbox: [], // top-level inbox task records, in order
    library: [], // top-level project and folder records, in order
    topTags: [],
    sequence: 0,
  };

  // What `settings.objectForKey` answers: OmniFocus's default times.
  F.SETTINGS = {
    DefaultDueTime: "17:00",
    DefaultStartTime: "00:00",
    DefaultPlannedTime: "09:00",
  };

  F.nextId = (prefix) => {
    F.db.sequence += 1;
    return "new-" + prefix + "-" + F.db.sequence;
  };

  const identifiers = new Map();
  F.identifier = (id, className) => {
    const key = className + ":" + id;
    if (!identifiers.has(key)) {
      identifiers.set(
        key,
        hostObject("ObjectIdentifier", {
          primaryKey: id,
          objectClass: className,
        }),
      );
    }
    return identifiers.get(key);
  };
  F.urlFor = (kind, id) => "omnifocus:///" + kind + "/" + id;

  // A location such as `task.ending`: opaque to scripts. `container` is a
  // record or one of "inbox", "library", "tags"; `sibling` anchors
  // `before`/`after`.
  F.location = (kind, container, sibling) => {
    const raw = {};
    raw[INTERNAL] = { location: true, kind, container, sibling };
    return hostObject("Location", raw, {});
  };

  F.insertInto = (list, item, kind, sibling) => {
    if (kind === "beginning") list.unshift(item);
    else if (kind === "before" || kind === "after") {
      const index = list.indexOf(sibling);
      list.splice(kind === "before" ? index : index + 1, 0, item);
    } else list.push(item);
  };
  F.removeFrom = (list, item) => {
    const index = list.indexOf(item);
    if (index >= 0) list.splice(index, 1);
  };
  F.findNamed = (list, name) => {
    const found = list.find((item) => item.name === name);
    return found ? found.host : null;
  };

  // Members that read and write a field of `record` directly.
  F.fieldAccessors = (raw, record, names) => {
    names.forEach((name) => {
      Object.defineProperty(raw, name, {
        get: () => record[name],
        set: (value) => {
          record[name] = value;
        },
        enumerable: true,
      });
    });
  };
  // Read-only members computed by the functions of `table`.
  F.getters = (raw, table) => {
    Object.keys(table).forEach((name) => {
      Object.defineProperty(raw, name, { get: table[name], enumerable: true });
    });
  };
  // A member with a validating setter.
  F.accessor = (raw, name, get, set) => {
    Object.defineProperty(raw, name, { get, set, enumerable: true });
  };

  // The functions behind `Task`, `Project`, `Folder` and `Tag`: instances
  // inherit from their prototypes, so `instanceof` works.
  F.TaskConstructor = function Task() {};
  F.ProjectConstructor = function Project() {};
  F.FolderConstructor = function Folder() {};
  F.TagConstructor = function Tag() {};
})(globalThis.__omnijsFake);
