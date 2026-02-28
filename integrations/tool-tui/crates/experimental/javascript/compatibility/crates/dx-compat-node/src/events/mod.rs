//! Event emitter pattern.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Listener function type.
pub type Listener = Box<dyn Fn(&[String]) + Send + Sync>;

/// EventEmitter implementation.
pub struct EventEmitter {
    listeners: Arc<RwLock<HashMap<String, Vec<ListenerEntry>>>>,
    max_listeners: usize,
}

struct ListenerEntry {
    listener: Listener,
    once: bool,
}

impl EventEmitter {
    /// Create a new EventEmitter.
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(RwLock::new(HashMap::new())),
            max_listeners: 10,
        }
    }

    /// Register an event listener.
    pub fn on(&self, event: &str, listener: Listener) {
        let mut listeners = self.listeners.write();
        let entries = listeners.entry(event.to_string()).or_default();

        if entries.len() >= self.max_listeners {
            eprintln!(
                "Warning: Possible EventEmitter memory leak detected. {} listeners added for event '{}'",
                entries.len() + 1,
                event
            );
        }

        entries.push(ListenerEntry {
            listener,
            once: false,
        });
    }

    /// Register a one-time listener.
    pub fn once(&self, event: &str, listener: Listener) {
        let mut listeners = self.listeners.write();
        let entries = listeners.entry(event.to_string()).or_default();
        entries.push(ListenerEntry {
            listener,
            once: true,
        });
    }

    /// Emit an event to all listeners.
    pub fn emit(&self, event: &str, args: &[String]) -> bool {
        let mut listeners = self.listeners.write();

        if let Some(entries) = listeners.get_mut(event) {
            let mut to_remove = Vec::new();

            for (i, entry) in entries.iter().enumerate() {
                (entry.listener)(args);
                if entry.once {
                    to_remove.push(i);
                }
            }

            // Remove once listeners in reverse order
            for i in to_remove.into_iter().rev() {
                entries.remove(i);
            }

            true
        } else {
            false
        }
    }

    /// Remove all listeners for an event.
    pub fn remove_all_listeners(&self, event: Option<&str>) {
        let mut listeners = self.listeners.write();

        if let Some(event) = event {
            listeners.remove(event);
        } else {
            listeners.clear();
        }
    }

    /// Set max listeners before warning.
    pub fn set_max_listeners(&mut self, n: usize) {
        self.max_listeners = n;
    }

    /// Get max listeners.
    pub fn get_max_listeners(&self) -> usize {
        self.max_listeners
    }

    /// Get listener count for an event.
    pub fn listener_count(&self, event: &str) -> usize {
        let listeners = self.listeners.read();
        listeners.get(event).map(|e| e.len()).unwrap_or(0)
    }

    /// Get event names.
    pub fn event_names(&self) -> Vec<String> {
        let listeners = self.listeners.read();
        listeners.keys().cloned().collect()
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventEmitter {
    fn clone(&self) -> Self {
        Self {
            listeners: Arc::clone(&self.listeners),
            max_listeners: self.max_listeners,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_on_and_emit() {
        let emitter = EventEmitter::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        emitter.on(
            "test",
            Box::new(move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }),
        );

        emitter.emit("test", &[]);
        emitter.emit("test", &[]);

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_once() {
        let emitter = EventEmitter::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        emitter.once(
            "test",
            Box::new(move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }),
        );

        emitter.emit("test", &[]);
        emitter.emit("test", &[]);

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_remove_all_listeners() {
        let emitter = EventEmitter::new();

        emitter.on("test", Box::new(|_| {}));
        emitter.on("test", Box::new(|_| {}));

        assert_eq!(emitter.listener_count("test"), 2);

        emitter.remove_all_listeners(Some("test"));

        assert_eq!(emitter.listener_count("test"), 0);
    }
}
