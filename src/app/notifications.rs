use std::collections::VecDeque;
use std::time::{Duration, Instant};

const MAX_NOTIFICATIONS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    Success,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub kind: NotificationKind,
    pub message: String,
    pub persistent: bool,
    pub created: Instant,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct NotificationQueue {
    items: VecDeque<Notification>,
    duration: Duration,
}

impl Default for NotificationQueue {
    fn default() -> Self {
        Self {
            items: VecDeque::new(),
            duration: Duration::from_secs(4),
        }
    }
}

impl NotificationQueue {
    pub fn push(&mut self, kind: NotificationKind, message: impl Into<String>, persistent: bool) {
        self.items.push_back(Notification {
            kind,
            message: message.into(),
            persistent,
            created: Instant::now(),
            duration: self.duration,
        });
        while self.items.len() > MAX_NOTIFICATIONS {
            self.items.pop_front();
        }
    }

    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        let before = self.items.len();
        self.items
            .retain(|item| item.persistent || now.duration_since(item.created) < item.duration);
        self.items.len() != before
    }

    pub fn dismiss(&mut self) {
        self.items.pop_front();
    }

    pub fn dismiss_latest(&mut self) {
        self.items.pop_back();
    }

    pub fn items(&self) -> impl DoubleEndedIterator<Item = &Notification> {
        self.items.iter()
    }

    #[cfg(test)]
    pub fn push_with_age(&mut self, kind: NotificationKind, message: &str, age: Duration) {
        self.push(kind, message, false);
        if let Some(item) = self.items.back_mut() {
            item.created = Instant::now() - age;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NotificationKind, NotificationQueue};
    use std::time::Duration;

    #[test]
    fn queue_is_bounded_and_expired_items_are_removed() {
        let mut queue = NotificationQueue::default();
        for index in 0..10 {
            queue.push(NotificationKind::Info, index.to_string(), false);
        }
        assert_eq!(queue.items().count(), 8);
        queue.push_with_age(NotificationKind::Error, "critical", Duration::from_secs(10));
        queue.tick();
        assert_eq!(queue.items().count(), 7);
        assert!(queue.items().any(|item| item.message == "8"));
    }

    #[test]
    fn persistent_items_survive_expiration() {
        let mut queue = NotificationQueue::default();
        queue.push(NotificationKind::Error, "failure", true);
        queue.tick();
        assert_eq!(queue.items().count(), 1);
        queue.dismiss();
        assert_eq!(queue.items().count(), 0);
    }

    #[test]
    fn latest_notification_can_be_dismissed_without_removing_older_items() {
        let mut queue = NotificationQueue::default();
        queue.push(NotificationKind::Info, "older", false);
        queue.push(NotificationKind::Warning, "confirmation", true);

        queue.dismiss_latest();

        assert_eq!(queue.items().count(), 1);
        assert_eq!(
            queue.items().next().map(|item| item.message.as_str()),
            Some("older")
        );
    }
}
