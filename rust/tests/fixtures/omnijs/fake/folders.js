// The strict OmniJS fake: `Folder` objects, and where projects and folders
// (the reference's "sections") sit in the library.

(function installFolders(F) {
  "use strict";
  const {
    INTERNAL,
    FolderStatus,
    db,
    hosts,
    hostArray,
    location,
    getters,
    accessor,
  } = F;

  F.sectionsOf = (folder) => (folder ? folder.sections : db.library);
  F.flattenSections = (list) =>
    list.flatMap((item) =>
      item.kind === "folder"
        ? [item, ...F.flattenSections(item.sections)]
        : [item],
    );

  // Where a project or folder goes: a folder location, a Folder, or nothing
  // for the end of the library.
  function sectionDestination(position) {
    if (position === null || position === undefined)
      return { parent: null, kind: "ending" };
    const target = F.record(position);
    if (
      target &&
      target.location &&
      (target.container === "library" || target.container.kind === "folder")
    ) {
      const parent = target.container === "library" ? null : target.container;
      return { parent, kind: target.kind, sibling: target.sibling };
    }
    if (target && target.kind === "folder")
      return { parent: target, kind: "ending" };
    throw new TypeError(
      "OmniJS strict fake: not a folder location: " + position,
    );
  }

  F.placeSection = (item, position) => {
    const { parent, kind, sibling } = sectionDestination(position);
    for (let ancestor = parent; ancestor; ancestor = ancestor.parent) {
      if (ancestor === item)
        throw new Error("OmniJS strict fake: a folder cannot move into itself");
    }
    const field = item.kind === "folder" ? "parent" : "parentFolder";
    F.removeFrom(F.sectionsOf(item[field]), item);
    item[field] = parent;
    F.insertInto(F.sectionsOf(parent), item, kind, sibling);
  };

  function folderState(raw, folder) {
    F.fieldAccessors(raw, folder, ["name"]);
    accessor(
      raw,
      "status",
      () => folder.status,
      (value) => {
        if (value !== FolderStatus.Active && value !== FolderStatus.Dropped) {
          throw new TypeError("status must be a Folder.Status");
        }
        folder.status = value;
      },
    );
    accessor(
      raw,
      "active",
      () => folder.status === FolderStatus.Active,
      (value) => {
        folder.status = value ? FolderStatus.Active : FolderStatus.Dropped;
      },
    );
    getters(raw, {
      id: () => F.identifier(folder.id, "Folder"),
      url: () => F.urlFor("folder", folder.id),
      added: () => F.now(),
      modified: () => F.now(),
      effectiveActive: () => {
        for (let item = folder; item; item = item.parent) {
          if (item.status !== FolderStatus.Active) return false;
        }
        return true;
      },
    });
  }

  function folderContents(raw, folder) {
    const projects = () =>
      folder.sections.filter((item) => item.kind === "project");
    const folders = () =>
      folder.sections.filter((item) => item.kind === "folder");
    const flattened = () => F.flattenSections(folder.sections);
    const sections = () => hostArray("SectionArray", hosts(folder.sections));
    getters(raw, {
      parent: () => (folder.parent ? folder.parent.host : null),
      projects: () => hostArray("ProjectArray", hosts(projects())),
      folders: () => hostArray("FolderArray", hosts(folders())),
      sections,
      children: sections,
      flattenedProjects: () =>
        hostArray(
          "ProjectArray",
          hosts(flattened().filter((i) => i.kind === "project")),
        ),
      flattenedFolders: () =>
        hostArray(
          "FolderArray",
          hosts(flattened().filter((i) => i.kind === "folder")),
        ),
      flattenedSections: () => hostArray("SectionArray", hosts(flattened())),
      flattenedChildren: () => hostArray("SectionArray", hosts(flattened())),
      beginning: () => location("beginning", folder),
      ending: () => location("ending", folder),
      before: () => location("before", folder.parent || "library", folder),
      after: () => location("after", folder.parent || "library", folder),
    });
    Object.assign(raw, {
      folderNamed: (name) => F.findNamed(folders(), name),
      projectNamed: (name) => F.findNamed(projects(), name),
      sectionNamed: (name) => F.findNamed(folder.sections, name),
      childNamed: (name) => F.findNamed(folder.sections, name),
    });
  }

  F.createFolderRecord = (fields) => {
    const folder = {
      kind: "folder",
      id: fields.id || F.nextId("folder"),
      name: fields.name,
      status: fields.status || FolderStatus.Active,
      parent: null,
      sections: [],
    };
    const raw = Object.create(F.FolderConstructor.prototype);
    raw[INTERNAL] = folder;
    folderState(raw, folder);
    folderContents(raw, folder);
    folder.host = F.hostObject("Folder", raw);
    db.folders.push(folder);
    return folder;
  };
})(globalThis.__omnijsFake);
