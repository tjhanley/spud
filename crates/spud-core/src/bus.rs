use std::collections::VecDeque;

use crate::event::Event;

/// A simple FIFO event queue.
///
/// The app loop uses the bus in a three-phase cycle:
/// 1. **Publish** — input polling and timers push events into the queue.
/// 2. **Drain** — all pending events are pulled out in order.
/// 3. **Broadcast** — each event is dispatched to modules via the registry.
pub struct EventBus {
    queue: VecDeque<Event>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Create an empty event bus.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Enqueue an event at the back of the queue.
    pub fn publish(&mut self, event: Event) {
        self.queue.push_back(event);
    }

    /// Remove and return all pending events, preserving insertion order.
    pub fn drain(&mut self) -> Vec<Event> {
        self.queue.drain(..).collect()
    }

    /// Return `true` if the queue contains at least one event.
    pub fn has_pending(&self) -> bool {
        !self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn publish_enqueues_events() {
        let mut bus = EventBus::new();
        bus.publish(Event::Tick {
            now: Instant::now(),
        });
        bus.publish(Event::Quit);
        assert!(bus.has_pending());
    }

    #[test]
    fn drain_returns_all_and_empties() {
        let mut bus = EventBus::new();
        bus.publish(Event::Tick {
            now: Instant::now(),
        });
        bus.publish(Event::Quit);
        let events = bus.drain();
        assert_eq!(events.len(), 2);
        assert!(!bus.has_pending());
    }

    #[test]
    fn drain_on_empty_returns_empty() {
        let mut bus = EventBus::new();
        let events = bus.drain();
        assert!(events.is_empty());
    }

    #[test]
    fn has_pending_correctness() {
        let mut bus = EventBus::new();
        assert!(!bus.has_pending());
        bus.publish(Event::Quit);
        assert!(bus.has_pending());
        bus.drain();
        assert!(!bus.has_pending());
    }

    #[test]
    fn preserves_order() {
        let mut bus = EventBus::new();
        bus.publish(Event::ModuleActivated { id: "a".into() });
        bus.publish(Event::ModuleDeactivated { id: "b".into() });
        bus.publish(Event::Quit);
        let events = bus.drain();
        assert!(matches!(&events[0], Event::ModuleActivated { id } if id == "a"));
        assert!(matches!(&events[1], Event::ModuleDeactivated { id } if id == "b"));
        assert!(matches!(&events[2], Event::Quit));
    }
}
