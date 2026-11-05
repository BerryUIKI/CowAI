use cowbot_core::error::CowResult;
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
    pub fn publish(&self, event: Event) -> CowResult<usize> {
        debug!(event_id = %event.id, "Publishing event to bus");

        // Record in history buffer
        {
            let mut hist = self.history.write();
            if hist.len() >= self.max_history {
                hist.pop_front();
            }
            hist.push_back(event.clone());
        }

        // Broadcast to live subscribers
        match self.sender.send(event) {
            Ok(receiver_count) => Ok(receiver_count),
            Err(_) => {
                // Err occurs if there are 0 active receivers, which is non-fatal for pub/sub
                Ok(0)
            }
        }
    }

    /// Subscribe to live events broadcast on this bus.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Retrieve a snapshot of recent historical events.
    pub fn history(&self) -> Vec<Event> {
        self.history.read().iter().cloned().collect()
    }

    /// Returns the number of active subscribers currently listening.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
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

        let recipients = bus.publish(event.clone()).unwrap();
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
            bus.publish(event).unwrap();
        }

        let history = bus.history();
        assert_eq!(history.len(), 3);
    }
}
