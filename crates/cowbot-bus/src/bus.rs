use cowbot_core::event::Event;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::debug;

/// Broadcast event bus for multi-agent messaging and pub/sub.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
    history: Arc<RwLock<VecDeque<Event>>>,
    max_history: usize,
}

impl EventBus {
    /// Create a new event bus with given broadcast channel capacity and history capacity.
    pub fn new(channel_capacity: usize, max_history: usize) -> Self {
        let (sender, _) = broadcast::channel(channel_capacity);
        Self {
            sender,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            max_history,
        }
    }

    /// Default bus with 1024 channel capacity and 1000 event history buffer.
    pub fn default_bus() -> Self {
        Self::new(1024, 1000)
    }

    /// Publish an event to all active subscribers and record to in-memory history.
    ///
    /// Returns the number of live subscribers that received the event.
    pub fn publish(&self, event: Event) -> usize {
        debug!(event_id = %event.id, "Publishing event to bus");

        if self.max_history > 0 {
            let mut hist = self.history.write();
            if hist.len() >= self.max_history {
                hist.pop_front();
            }
            hist.push_back(event.clone());
        }

        // A send error means there are no active receivers, which is fine for pub/sub.
        self.sender.send(event).unwrap_or(0)
    }

    /// Subscribe to live events broadcast on this bus.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Retrieve a snapshot of recent historical events.
    pub fn history(&self) -> Vec<Event> {
        self.history.read().iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cowbot_core::event::{Event, EventPayload};

    #[tokio::test]
    async fn test_bus_publish_and_receive() {
        let bus = EventBus::new(16, 50);
        let mut rx = bus.subscribe();

        let event = Event::new(EventPayload::SystemNotice {
            message: "CowBot engine initialized".to_string(),
        });

        let recipients = bus.publish(event.clone());
        assert_eq!(recipients, 1);

        let received = rx.recv().await.unwrap();
        assert_eq!(received.id, event.id);

        let history = bus.history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, event.id);
    }

    #[tokio::test]
    async fn test_bus_history_capacity() {
        let bus = EventBus::new(16, 3);
        for i in 0..5 {
            let event = Event::new(EventPayload::SystemNotice {
                message: format!("Notice {}", i),
            });
            bus.publish(event);
        }

        let history = bus.history();
        assert_eq!(history.len(), 3);
    }

    #[tokio::test]
    async fn test_bus_with_zero_history_keeps_nothing() {
        let bus = EventBus::new(16, 0);
        bus.publish(Event::new(EventPayload::SystemNotice {
            message: "Notice".to_string(),
        }));

        assert!(bus.history().is_empty());
    }
}
