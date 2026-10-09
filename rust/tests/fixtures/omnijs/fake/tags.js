// The strict OmniJS fake: `Tag` objects and where they sit in the tag tree.

(function installTags(F) {
  "use strict";
  const {
    INTERNAL,
    TagStatus,
    TaskStatus,
    db,
    hosts,
    hostArray,
    location,
    getters,
    accessor,
  } = F;
  const STATUSES = [TagStatus.Active, TagStatus.OnHold, TagStatus.Dropped];

  F.tagsUnder = (parent) => (parent ? parent.children : db.topTags);
  F.flattenTags = (list) =>
    list.flatMap((tag) => [tag, ...F.flattenTags(tag.children)]);

  // Where a tag goes: a tag location, a Tag, or nothing for the end of the
  // top-level tags.
  F.placeTag = (tag, position) => {
    let destination = { parent: null, kind: "ending" };
    if (position !== null && position !== undefined) {
      const target = F.record(position);
      if (
        target &&
        target.location &&
        (target.container === "tags" || target.container.kind === "tag")
      ) {
        const parent = target.container === "tags" ? null : target.container;
        destination = { parent, kind: target.kind, sibling: target.sibling };
      } else if (target && target.kind === "tag") {
        destination = { parent: target, kind: "ending" };
      } else {
        throw new TypeError(
          "OmniJS strict fake: not a tag location: " + position,
        );
      }
    }
    F.removeFrom(F.tagsUnder(tag.parent), tag);
    tag.parent = destination.parent;
    F.insertInto(
      F.tagsUnder(destination.parent),
      tag,
      destination.kind,
      destination.sibling,
    );
  };

  function tagState(raw, tag) {
    F.fieldAccessors(raw, tag, [
      "name",
      "allowsNextAction",
      "childrenAreMutuallyExclusive",
    ]);
    accessor(
      raw,
      "status",
      () => tag.status,
      (value) => {
        if (!STATUSES.includes(value))
          throw new TypeError("status must be a Tag.Status");
        tag.status = value;
      },
    );
    accessor(
      raw,
      "active",
      () => tag.status !== TagStatus.Dropped,
      (value) => {
        tag.status = value ? TagStatus.Active : TagStatus.Dropped;
      },
    );
    getters(raw, {
      id: () => F.identifier(tag.id, "Tag"),
      url: () => F.urlFor("tag", tag.id),
      added: () => F.now(),
      modified: () => F.now(),
      effectiveActive: () => {
        for (let item = tag; item; item = item.parent) {
          if (item.status === TagStatus.Dropped) return false;
        }
        return true;
      },
    });
  }

  function tagContents(raw, tag) {
    const tagged = () => db.tasks.filter((task) => task.tags.includes(tag));
    const children = () => hostArray("TagArray", hosts(tag.children));
    const descendants = () =>
      hostArray("TagArray", hosts(F.flattenTags(tag.children)));
    const available = (task) => F.taskStatus(task) === TaskStatus.Available;
    getters(raw, {
      parent: () => (tag.parent ? tag.parent.host : null),
      children,
      tags: children,
      flattenedTags: descendants,
      flattenedChildren: descendants,
      tasks: () => hostArray("TaskArray", hosts(tagged())),
      remainingTasks: () =>
        hostArray("TaskArray", hosts(tagged().filter(F.isRemaining))),
      availableTasks: () =>
        hostArray("TaskArray", hosts(tagged().filter(available))),
      projects: () =>
        hostArray(
          "ProjectArray",
          hosts(db.projects.filter((p) => p.root.tags.includes(tag))),
        ),
      beginning: () => location("beginning", tag),
      ending: () => location("ending", tag),
      before: () => location("before", tag.parent || "tags", tag),
      after: () => location("after", tag.parent || "tags", tag),
    });
    Object.assign(raw, {
      tagNamed: (name) => F.findNamed(tag.children, name),
      childNamed: (name) => F.findNamed(tag.children, name),
    });
  }

  F.createTagRecord = (fields) => {
    const tag = {
      kind: "tag",
      id: fields.id || F.nextId("tag"),
      name: fields.name,
      status: fields.status || TagStatus.Active,
      allowsNextAction: true,
      childrenAreMutuallyExclusive: false,
      parent: null,
      children: [],
    };
    const raw = Object.create(F.TagConstructor.prototype);
    raw[INTERNAL] = tag;
    tagState(raw, tag);
    tagContents(raw, tag);
    tag.host = F.hostObject("Tag", raw);
    db.tags.push(tag);
    return tag;
  };
})(globalThis.__omnijsFake);
