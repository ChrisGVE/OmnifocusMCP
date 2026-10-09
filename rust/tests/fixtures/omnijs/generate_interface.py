#!/usr/bin/env python3
"""Regenerates `interface.js`, the documented OmniJS surface the strict fake
in `fake/` enforces.

Usage (from `rust/`, macOS with OmniFocus 4 installed):

    python3 tests/fixtures/omnijs/generate_interface.py > tests/fixtures/omnijs/interface.js

Only files on disk are read; OmniFocus is never launched or scripted.

Source of truth: the API reference OmniFocus generates its Automation
documentation from, shipped in its bundle as `*-Interfaces.strings` files.
Each line is `"Class.member" = "description";`, where the member is a plain
name (a property or a method), `get_name` (readable) or `set_name`
(writable). A `get_` name with no matching `set_` is read-only.

The files name classes by their Objective-C names (`OFMContext` is the JS
`Tag`, `OFMAlarm` is `Task.Notification`, ...), and they do not say which
class inherits from which. Both mappings are therefore written out below.
The inheritance was read from the Objective-C class metadata of the
`OmniFocusModel` binary (`otool -arch arm64 -oV .../OmniFocusModel`,
`superclass` entries), and it agrees with the class headers of the
OmniFocus API reference:

    OFMTask, OFMContext, OFMFolder -> OFMActiveObject -> (OFMTreeObject ->
        OFMNamedObject ->) OFMDatedObject -> OFMObject
    OFMProjectInfo -> OFMObject
    OFMAlarm, OFMPerspective -> OFMDatedObject -> OFMObject

(`OFMTreeObject` and `OFMNamedObject` declare no scripting members.)
"""

import json
import re
import sys
from collections import defaultdict

APP = "/Applications/OmniFocus.app/Contents"
SOURCES = [
    f"{APP}/Frameworks/OmniFocusModel.framework/Versions/A/Resources/OmniFocusModel-Interfaces.strings",
    f"{APP}/Resources/OmniFocus-Interfaces.strings",
    f"{APP}/Frameworks/OmniJS.framework/Versions/A/Resources/OmniJS-Interfaces.strings",
]

# JS-facing instance class -> the documented classes whose members it has.
# "@Name" pulls in another entry of this table (the superclass).
INSTANCE_CLASSES = {
    "DatabaseObject": ["OFMObject"],
    "DatedObject": ["OFMDatedObject", "@DatabaseObject"],
    "ActiveObject": ["OFMActiveObject", "@DatedObject"],
    "Task": ["OFMTask", "@ActiveObject"],
    "Tag": ["OFMContext", "@ActiveObject"],
    "Folder": ["OFMFolder", "@ActiveObject"],
    "Project": ["OFMProjectInfo", "@DatabaseObject"],
    "Task.Notification": ["OFMAlarm", "@DatedObject"],
    "Task.RepetitionRule": ["OFMRepetitionRule"],
    "Project.ReviewInterval": ["ProjectReviewInterval"],
    "ObjectIdentifier": ["ODOObjectID"],
    "Perspective.Custom": ["OFMPerspective", "@DatedObject"],
    "Perspective.BuiltIn": ["OFMBuiltInPerspectiveIdentifier.Wrapper"],
    # The OmniJS global object inside OmniFocus is the database.
    "Database": ["OFMXMLDataStore"],
    # `document`: OmniFocus's document class plus the generic NSDocument API.
    "Document": ["__Document", "NSDocument"],
    "Settings": ["OFMSettingWrapper"],
    "Inbox": ["DataStoreInbox"],
    "Library": ["DataStoreLibrary"],
    "Tags": ["DataStoreTags"],
    "TaskArray": ["TaskArray"],
    "ProjectArray": ["ProjectArray"],
    "FolderArray": ["FolderArray"],
    "TagArray": ["TagArray"],
    "SectionArray": ["SectionArray"],
}

# Constructor objects and the global path they are reachable at.
CONSTRUCTORS = {
    "TaskConstructor": "Task",
    "ProjectConstructor": "Project",
    "FolderConstructor": "Folder",
    "TagConstructor": "Tag",
    "PerspectiveScriptingNamespaceConstructor": "Perspective",
    "PerspectiveConstructor": "Perspective.Custom",
    "RepetitionRuleConstructor": "Task.RepetitionRule",
    "ForecastDayConstructor": "ForecastDay",
    "TypeIdentifierConstructor": "TypeIdentifier",
}

ENUM_WRAPPER = re.compile(r"^_([A-Za-z]+(?:_[A-Za-z]+)*)_GeneratedWrapperConstructor$")


def read_members():
    """Maps each documented class name to {member: set of prefixes}."""
    members = defaultdict(lambda: defaultdict(set))
    for path in SOURCES:
        with open(path, encoding="utf-8") as handle:
            for key in re.findall(r'^"([^"]+)"\s*=', handle.read(), re.M):
                if "." not in key:
                    continue
                owner, member = key.rsplit(".", 1)
                prefix = ""
                if member.startswith("get_") or member.startswith("set_"):
                    prefix, member = member[:4], member[4:]
                members[owner][member].add(prefix)
    return members


def access(prefixes):
    """'ro' for a getter without a setter, otherwise 'rw'.

    A plain member may be a read-only property or a method; the reference
    does not say, so it counts as writable to avoid false alarms."""
    return "ro" if prefixes == {"get_"} else "rw"


def instance_members(members, js_class):
    result = {}
    for source in INSTANCE_CLASSES[js_class]:
        if source.startswith("@"):
            result.update(instance_members(members, source[1:]))
            continue
        if source not in members:
            raise SystemExit(
                f"documented class {source} not found in the interface files"
            )
        for name, prefixes in members[source].items():
            result[name] = access(prefixes)
    return result


def statics_and_enums(members):
    """The documented statics of each constructor or namespace path, and the
    values of each enum."""
    statics = defaultdict(set)
    enums = {}
    for objc_name, path in CONSTRUCTORS.items():
        statics[path].update(
            name for name in members.get(objc_name, {}) if name != "constructInstance"
        )
    for owner in list(members):
        match = ENUM_WRAPPER.match(owner)
        if match:
            path = match.group(1).replace("_", ".")
            enums[path] = sorted(members[owner])
            statics.setdefault(path, set())
    # Every nested path makes its last segment a static of its parent:
    # `Task.Notification.Kind` puts `Kind` on `Task.Notification` and
    # `Notification` on `Task`.
    for path in list(statics) + list(enums):
        parts = path.split(".")
        for depth in range(1, len(parts)):
            statics[".".join(parts[:depth])].add(parts[depth])
    return statics, enums


def main():
    members = read_members()
    instances = {
        js_class: dict(sorted(instance_members(members, js_class).items()))
        for js_class in INSTANCE_CLASSES
    }
    statics, enums = statics_and_enums(members)
    fixture = {
        "instances": instances,
        "statics": {path: sorted(names) for path, names in sorted(statics.items())},
        "enums": dict(sorted(enums.items())),
    }
    print(
        "// GENERATED by generate_interface.py from OmniFocus's *-Interfaces.strings;"
    )
    print(
        "// do not edit by hand. See rust/README.md, Testing, for how to regenerate it."
    )
    # An assignment, not `var`, so the fake can delete it after reading it.
    print("globalThis.__OMNIJS_INTERFACE = " + json.dumps(fixture, indent=1) + ";")


if __name__ == "__main__":
    sys.exit(main())
