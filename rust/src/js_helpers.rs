//! OmniJS source snippets shared by the tool script builders.
//!
//! Every tool builds its OmniJS script as a string (see `crate::tools`), and
//! `crate::jxa` runs it inside OmniFocus. Logic that more than one script
//! needs lives here once, as JavaScript source, and is prepended by each
//! builder that uses it — so the behaviour has a single source of truth.

/// Date parsing helpers for user-supplied date strings (upstream issue #13).
///
/// Plain `new Date(text)` follows ECMAScript: a bare `"2026-07-12"` is read
/// as UTC midnight, so it lands on the wrong local time (or the wrong day)
/// everywhere outside UTC. These helpers read a bare `YYYY-MM-DD` as a local
/// calendar day instead, and reject impossible days such as `2026-02-30`
/// rather than letting them roll over or become an Invalid Date.
///
/// Defined functions:
/// - `parseLocalDate(value, fieldName)`: a bare date is local midnight; any
///   other string goes through `new Date` (ISO date-times with `Z` or an
///   offset keep their instant, without an offset they are local). Throws
///   `"<fieldName> must be YYYY-MM-DD or an ISO 8601 date-time; received <value>"`
///   on anything unparseable.
/// - `parseOptionalLocalDate(value, fieldName)`: `null`/`undefined` → `null`
///   (an absent filter bound), otherwise `parseLocalDate`.
/// - `parseWriteDate(value, fieldName, defaultTimeKey, factoryDefault)`: for
///   values written to OmniFocus. A bare date becomes that local day at the
///   user's configured default time — read from OmniFocus's
///   `settings.objectForKey(defaultTimeKey)` (`"DefaultDueTime"`,
///   `"DefaultStartTime"`) — or at `factoryDefault` (`"H:MM"`/`"HH:MM"`) when
///   the setting is unavailable or not a valid time. Other values go through
///   `parseLocalDate`.
///
/// Internal helpers are prefixed `ofDate` to avoid colliding with names in
/// the scripts they are prepended to.
pub const JS_DATE_HELPERS: &str = r#"function ofDateInvalidError(value, fieldName) {
  return new Error(fieldName + " must be YYYY-MM-DD or an ISO 8601 date-time; received " + JSON.stringify(value));
}
function ofDateMatchBareDay(text) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(text);
  if (match === null) return null;
  return { year: Number(match[1]), monthIndex: Number(match[2]) - 1, day: Number(match[3]) };
}
function ofDateLocalDayAt(value, fieldName, bareDay, hours, minutes) {
  // setFullYear keeps years below 100 literal (new Date(50, 0, 1) is 1950).
  const result = new Date(2000, 0, 1, 0, 0, 0, 0);
  result.setFullYear(bareDay.year, bareDay.monthIndex, bareDay.day);
  result.setHours(hours, minutes, 0, 0);
  const sameDay = result.getFullYear() === bareDay.year
    && result.getMonth() === bareDay.monthIndex
    && result.getDate() === bareDay.day;
  if (!sameDay) throw ofDateInvalidError(value, fieldName);
  return result;
}
function ofDateParseClockTime(value) {
  if (value === null || value === undefined) return null;
  const match = /^(\d{1,2}):(\d{2})$/.exec(String(value).trim());
  if (match === null) return null;
  const hours = Number(match[1]);
  const minutes = Number(match[2]);
  if (hours > 23 || minutes > 59) return null;
  return { hours: hours, minutes: minutes };
}
function ofDateConfiguredDefaultTime(defaultTimeKey) {
  if (typeof settings === "undefined" || settings === null) return null;
  try {
    return ofDateParseClockTime(settings.objectForKey(defaultTimeKey));
  } catch (error) {
    return null;
  }
}
function parseLocalDate(value, fieldName) {
  if (typeof value !== "string") throw ofDateInvalidError(value, fieldName);
  const text = value.trim();
  const bareDay = ofDateMatchBareDay(text);
  if (bareDay !== null) return ofDateLocalDayAt(value, fieldName, bareDay, 0, 0);
  const parsed = new Date(text);
  if (Number.isNaN(parsed.getTime())) throw ofDateInvalidError(value, fieldName);
  return parsed;
}
function parseOptionalLocalDate(value, fieldName) {
  if (value === null || value === undefined) return null;
  return parseLocalDate(value, fieldName);
}
function parseWriteDate(value, fieldName, defaultTimeKey, factoryDefault) {
  const bareDay = typeof value === "string" ? ofDateMatchBareDay(value.trim()) : null;
  if (bareDay === null) return parseLocalDate(value, fieldName);
  const factoryTime = ofDateParseClockTime(factoryDefault);
  if (factoryTime === null) {
    throw new Error("invalid factory default time for " + defaultTimeKey + ": " + JSON.stringify(factoryDefault));
  }
  const time = ofDateConfiguredDefaultTime(defaultTimeKey) || factoryTime;
  return ofDateLocalDayAt(value, fieldName, bareDay, time.hours, time.minutes);
}
"#;

/// Project status naming (upstream #10).
///
/// `project.status` is a `Project.Status` enum value. Its string form is
/// `"[object Project.Status: Done]"`, so matching on text never saw
/// "completed" and every completed project was reported as active. This
/// compares against the enum members themselves:
///
/// | `Project.Status` | reported as   |
/// | ---------------- | ------------- |
/// | `Active`         | `"active"`    |
/// | `Done`           | `"completed"` |
/// | `Dropped`        | `"dropped"`   |
/// | `OnHold`         | `"on_hold"`   |
/// | anything else    | `"unknown"`   |
///
/// The fallback is deliberately `"unknown"` rather than `"active"`: a status
/// this code does not recognise must show up as such, not as wrong data.
///
/// Defined function: `normalizeProjectStatus(project)`.
pub const JS_PROJECT_STATUS: &str = r#"function normalizeProjectStatus(project) {
  const status = project.status;
  if (status === Project.Status.Active) return "active";
  if (status === Project.Status.Done) return "completed";
  if (status === Project.Status.Dropped) return "dropped";
  if (status === Project.Status.OnHold) return "on_hold";
  return "unknown";
}
"#;

/// Project review interval helpers (upstream #12).
///
/// `project.reviewInterval` is a `Project.ReviewInterval` *value object*:
/// reading it returns a copy, and OmniFocus only accepts that type back. The
/// documented way to change it is to read it, set `steps` and `unit`, and
/// assign it back — assigning a plain `{steps, unit}` object is rejected. Its
/// string form is "[object Project.ReviewInterval]", so it must be formatted
/// field by field.
///
/// Defined functions:
/// - `formatReviewInterval(interval)`: `"<steps> <unit>"` (e.g. `"2 weeks"`),
///   or `null` when the project has no interval.
/// - `updatedReviewInterval(project, requested)`: the project's interval with
///   `steps` and `unit` taken from `requested` (already validated in Rust, see
///   `crate::review_interval`). Nothing changes on the project until the
///   caller assigns the result. Throws `"Project has no review interval to
///   update"` when the project has none to start from.
pub const JS_REVIEW_INTERVAL: &str = r#"function formatReviewInterval(interval) {
  if (interval === null || interval === undefined) return null;
  return interval.steps + " " + interval.unit;
}
function updatedReviewInterval(project, requested) {
  const interval = project.reviewInterval;
  if (!interval) throw new Error("Project has no review interval to update");
  interval.steps = requested.steps;
  interval.unit = requested.unit;
  return interval;
}
"#;
