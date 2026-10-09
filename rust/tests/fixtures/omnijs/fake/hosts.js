// The strict OmniJS fake: the Proxies every host object is made of. See
// `strict.js` for what a violation and a gap are.

(function installHosts(F) {
  "use strict";
  const { own, violation, gap, INTERNAL } = F;
  const OBJECT_INTRINSICS = new Set([
    "toString",
    "toLocaleString",
    "valueOf",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "constructor",
  ]);
  const ARRAY_MEMBERS = new Set(Object.getOwnPropertyNames(Array.prototype));
  const FUNCTION_MEMBERS = new Set([
    "name",
    "length",
    "prototype",
    "call",
    "apply",
    "bind",
    "toString",
  ]);
  const isIndex = (key) => /^(0|[1-9][0-9]*)$/.test(key);

  function readMember(cls, allowed, target, key, receiver) {
    if (!own(allowed, key)) {
      if (key === "toJSON")
        throw violation(
          "a " + cls + " host object was passed to JSON.stringify",
        );
      throw violation("read of undocumented " + cls + "." + key);
    }
    if (!(key in target)) {
      if (allowed[key] === "probe") return undefined;
      throw gap(cls + "." + key);
    }
    return Reflect.get(target, key, receiver);
  }

  function writeMember(cls, allowed, target, key, value) {
    if (!own(allowed, key))
      throw violation("write of undocumented " + cls + "." + key);
    if (allowed[key] === "ro")
      throw violation("write of read-only " + cls + "." + key);
    if (!(key in target) && allowed[key] !== "probe")
      throw gap("writing " + cls + "." + key);
    // Scripts run in sloppy mode, where a failed assignment is silent; the
    // fake must not let one pass unnoticed.
    if (!Reflect.set(target, key, value))
      throw gap("writing " + cls + "." + key);
    return true;
  }

  // An instance of `cls`. `members` maps a member name to "rw" or "ro" and
  // defaults to the documented members; "probe" marks a name the production
  // wrapper feature-tests and may set (see `document` in `globals.js`).
  F.hostObject = (cls, raw, members) => {
    const allowed = members || F.documentedMembers(cls);
    if (!own(raw, "toString")) {
      Object.defineProperty(raw, "toString", {
        value: () => "[object " + cls + "]",
      });
    }
    return new Proxy(raw, {
      get(target, key, receiver) {
        if (typeof key === "symbol" || OBJECT_INTRINSICS.has(key)) {
          return Reflect.get(target, key, receiver);
        }
        return readMember(cls, allowed, target, key, receiver);
      },
      set(target, key, value) {
        if (typeof key === "symbol") return Reflect.set(target, key, value);
        return writeMember(cls, allowed, target, key, value);
      },
      has: (target, key) =>
        typeof key === "symbol"
          ? key in target
          : own(allowed, key) && key in target,
      deleteProperty: (_target, key) => {
        throw violation("delete of " + cls + "." + String(key));
      },
      defineProperty: (_target, key) => {
        throw violation("defineProperty of " + cls + "." + String(key));
      },
    });
  };

  // An Array the reference documents (`TaskArray` with `byName`, `Inbox`
  // with `beginning`/`ending`, ...), or a plain `Array` of host objects.
  // Scripts get a copy, as from OmniJS. `extras` holds the class's members.
  F.hostArray = (cls, items, extras) => {
    const allowed = cls === "Array" ? {} : F.documentedMembers(cls);
    const target = items.slice();
    if (own(allowed, "byName")) {
      Object.defineProperty(target, "byName", {
        value: (name) =>
          target.find((item) => item[INTERNAL].name === name) || null,
      });
    }
    if (extras)
      Object.defineProperties(target, Object.getOwnPropertyDescriptors(extras));
    return new Proxy(target, {
      get(array, key, receiver) {
        if (
          typeof key === "symbol" ||
          key === "length" ||
          isIndex(key) ||
          ARRAY_MEMBERS.has(key)
        ) {
          return Reflect.get(array, key, receiver);
        }
        return readMember(cls, allowed, array, key, receiver);
      },
      set(array, key, value) {
        if (key === "length" || isIndex(key))
          return Reflect.set(array, key, value);
        throw violation("write of undocumented " + cls + "." + String(key));
      },
    });
  };

  // A namespace such as `Task.Notification`: only its documented statics,
  // or `allowedNames` when given, may be read.
  F.hostNamespace = (path, raw, allowedNames) => {
    const allowed = allowedNames || new Set(F.INTERFACE.statics[path] || []);
    return new Proxy(raw, {
      get(target, key, receiver) {
        if (typeof key === "symbol" || OBJECT_INTRINSICS.has(key)) {
          return Reflect.get(target, key, receiver);
        }
        if (!allowed.has(key))
          throw violation("read of undocumented " + path + "." + key);
        if (!(key in target)) throw gap(path + "." + key);
        return Reflect.get(target, key, receiver);
      },
      set: (_target, key) => {
        throw violation("write of " + path + "." + String(key));
      },
    });
  };

  // A constructor such as `Task`. `target` is the function whose `prototype`
  // instances inherit from, so `task instanceof Task` works as in OmniJS.
  F.hostConstructor = (path, construct, statics, target) => {
    const allowed = new Set(F.INTERFACE.statics[path] || []);
    return new Proxy(target || function () {}, {
      get(fn, key, receiver) {
        if (typeof key === "symbol" || FUNCTION_MEMBERS.has(key))
          return Reflect.get(fn, key, receiver);
        if (!allowed.has(key))
          throw violation("read of undocumented " + path + "." + key);
        if (!own(statics, key)) throw gap(path + "." + key);
        return statics[key];
      },
      set: (_fn, key) => {
        throw violation("write of " + path + "." + String(key));
      },
      construct: (_fn, args) => construct(...args),
      apply: () => {
        throw violation("calling " + path + " without new");
      },
    });
  };

  // An enum such as `Task.Status`: its values and `all`. Values are opaque
  // unless `makeValue` builds them (built-in perspectives have a `name`).
  F.makeEnum = (path, makeValue) => {
    const names = F.INTERFACE.enums[path];
    if (!names) throw new Error("harness: no documented enum " + path);
    const raw = {};
    const values = names
      .filter((name) => name !== "all")
      .map((name) => {
        raw[name] = makeValue
          ? makeValue(name)
          : F.hostObject(
              path,
              { toString: () => "[object " + path + ": " + name + "]" },
              {},
            );
        return raw[name];
      });
    if (names.includes("all")) raw.all = F.hostArray("Array", values);
    return F.hostNamespace(
      path,
      raw,
      new Set([...names, ...(F.INTERFACE.statics[path] || [])]),
    );
  };
})(globalThis.__omnijsFake);
