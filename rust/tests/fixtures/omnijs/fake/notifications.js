// The strict OmniJS fake: `Task.Notification` objects, and adding or
// removing them on a task or project.

(function installNotifications(F) {
  "use strict";
  const { INTERNAL, NotificationKind, Date, RealDate, FIXED_NOW } = F;

  function createNotification(task, kind, fireDate, offset, id) {
    const notification = {
      kind: "notification",
      id: id || F.nextId("notification"),
      task,
    };
    const raw = {};
    raw[INTERNAL] = notification;
    F.getters(raw, {
      id: () => F.identifier(notification.id, "Task.Notification"),
      url: () => F.urlFor("notification", notification.id),
      added: () => F.now(),
      modified: () => F.now(),
      kind: () => kind,
      initialFireDate: () => fireDate,
      nextFireDate: () => (fireDate.getTime() > FIXED_NOW ? fireDate : null),
      isSnoozed: () => false,
      repeatInterval: () => 0,
      task: () => task.host,
      usesFloatingTimeZone: () => false,
      // "Getting or setting this property throws an error if this
      // notification's kind is not absolute." (Likewise for relative.)
      absoluteFireDate: () => {
        if (kind !== NotificationKind.Absolute)
          throw new Error("absoluteFireDate: not an absolute notification");
        return fireDate;
      },
      relativeFireOffset: () => {
        if (kind !== NotificationKind.DueRelative)
          throw new Error("relativeFireOffset: not a relative notification");
        return offset;
      },
    });
    notification.host = F.hostObject("Task.Notification", raw);
    return notification;
  }

  // "Supplying a Date creates an absolute notification that will fire at
  // that date. Supplying a Double will create a due-relative notification.
  // Specifying a due relative notification when this task's effectiveDueDate
  // is not set will result in an error." The offset is in minutes
  // (`relativeFireOffset`).
  F.addNotificationTo = (task, info, id) => {
    let notification;
    if (info instanceof RealDate) {
      notification = createNotification(
        task,
        NotificationKind.Absolute,
        info,
        null,
        id,
      );
    } else if (typeof info === "number") {
      const due = F.effectiveDueDate(task);
      if (!due)
        throw new Error(
          "Cannot add a due-relative notification to a task without a due date",
        );
      const fireDate = new Date(due.getTime() + info * 60000);
      notification = createNotification(
        task,
        NotificationKind.DueRelative,
        fireDate,
        info,
        id,
      );
    } else {
      throw new TypeError("addNotification expects a Date or a Number");
    }
    task.notifications.push(notification);
    return notification.host;
  };

  // "Supplying a notification that is not in this task's notifications
  // array ... results in an error."
  F.removeNotificationFrom = (task, host) => {
    const notification = F.record(host);
    if (!notification || !task.notifications.includes(notification)) {
      throw new Error("Notification does not belong to this task");
    }
    F.removeFrom(task.notifications, notification);
  };
})(globalThis.__omnijsFake);
