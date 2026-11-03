// The strict OmniJS fake, first part: what the fake is, and the strictness
// itself.
//
// The fake stands in for the OmniJS context OmniFocus runs our scripts in
// (see `tests/support/omnijs_harness.rs`, which evaluates `interface.js`,
// then these files in the order the harness lists, then `scenario.js`, in a
// JavaScriptCore realm of their own). Every object a script can reach is a
// Proxy that checks each property access against `interface.js`, generated
// from OmniFocus's API reference:
//
// - reading or writing a member the reference does not document for the
//   object's class is a VIOLATION, logged in `violations` and thrown. The
//   log matters because a script may catch the throw
//   (`detectPlannedDateSupport` does), and the harness must still see it;
// - writing a member documented as read-only is a violation too;
// - reading a documented member the fake does not model is a GAP, logged in
//   `gaps` and thrown, so the fake never answers `undefined` where OmniFocus
//   would return a value.
//
// Globals are strict by construction: only documented `Database` members
// and constructors are defined, so any other free name is a ReferenceError.
//
// The files share their functions through `globalThis.__omnijsFake`, which
// the last one (`globals.js`) removes. Where OmniFocus's behaviour is not
// documented the fake makes the plainest choice and says so.

(function installStrictness(global) {
  "use strict";
  const F = {};
  Object.defineProperty(global, "__omnijsFake", {
    value: F,
    configurable: true,
  });

  F.INTERFACE = global.__OMNIJS_INTERFACE;
  const config = global.__HARNESS_CONFIG;
  delete global.__OMNIJS_INTERFACE;
  delete global.__HARNESS_CONFIG;

  // The jsc shell's own helpers are not OmniJS globals.
  const shellGlobals = [
    "print",
    "printErr",
    "debug",
    "describe",
    "describeArray",
    "load",
    "loadString",
    "run",
    "runString",
    "readFile",
    "read",
    "readline",
    "checkSyntax",
    "quit",
    "gc",
    "fullGC",
    "edenGC",
    "gcHeapSize",
    "version",
    "jscStack",
    "sleepSeconds",
    "preciseTime",
    "createGlobalObject",
    "setTimeout",
    "drainMicrotasks",
    "$262",
    "$",
    "$vm",
  ];
  shellGlobals.forEach((name) => {
    Reflect.deleteProperty(global, name);
    if (name in global)
      throw new Error("harness: cannot remove the jsc shell global " + name);
  });

  F.violations = [];
  F.gaps = [];
  F.violation = (text) => {
    F.violations.push(text);
    return new Error("OmniJS strict fake: " + text);
  };
  F.gap = (text) => {
    F.gaps.push(text);
    return new Error("OmniJS strict fake does not model " + text);
  };

  // `new Date()` and `Date.now()` return the configured instant, so results
  // do not depend on when the test runs. The time zone is the process's TZ.
  const RealDate = global.Date;
  const fixedNow = new RealDate(config.now).getTime();
  if (Number.isNaN(fixedNow))
    throw new Error("harness: invalid fixed clock " + config.now);
  class FixedClockDate extends RealDate {
    constructor(...args) {
      if (args.length === 0) super(fixedNow);
      else super(...args);
    }
    static now() {
      return fixedNow;
    }
  }
  Object.defineProperty(FixedClockDate, "name", { value: "Date" });
  global.Date = FixedClockDate;
  Object.assign(F, { RealDate, Date: FixedClockDate, FIXED_NOW: fixedNow });
  F.AUDIT_FIXTURES = config.auditFixtures === true;
  F.now = () => new FixedClockDate();

  F.INTERNAL = Symbol("fake internal record");
  F.own = (object, key) => Object.prototype.hasOwnProperty.call(object, key);
  F.record = (host) =>
    (host !== null && typeof host === "object") || typeof host === "function"
      ? host[F.INTERNAL]
      : undefined;
  F.hosts = (records) => records.map((item) => item.host);
  F.documentedMembers = (cls) => {
    const members = F.INTERFACE.instances[cls];
    if (!members) throw new Error("harness: no documented class " + cls);
    return members;
  };
})(globalThis);
