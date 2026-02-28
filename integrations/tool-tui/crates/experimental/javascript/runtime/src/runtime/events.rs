//! EventEmitter implementation

use std::collections::HashMap;

type EventCallback = Box<dyn FnMut(&[u8])>;

pub struct EventEmitter {
    listeners: HashMap<String, Vec<EventCallback>>,
    once_listeners: HashMap<String, Vec<EventCallback>>,
}

impl EventEmitter {
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
            once_listeners: HashMap::new(),
        }
    }

    pub fn on<F>(&mut self, event: &str, callback: F)
    where
        F: FnMut(&[u8]) + 'static,
    {
        self.listeners.entry(event.to_string()).or_default().push(Box::new(callback));
    }

    pub fn once<F>(&mut self, event: &str, callback: F)
    where
        F: FnMut(&[u8]) + 'static,
    {
        self.once_listeners
            .entry(event.to_string())
            .or_default()
            .push(Box::new(callback));
    }

    pub fn emit(&mut self, event: &str, data: &[u8]) {
        if let Some(listeners) = self.listeners.get_mut(event) {
            for callback in listeners.iter_mut() {
                callback(data);
            }
        }

        if let Some(listeners) = self.once_listeners.remove(event) {
            for mut callback in listeners {
                callback(data);
            }
        }
    }

    pub fn remove_listener(&mut self, event: &str) {
        self.listeners.remove(event);
        self.once_listeners.remove(event);
    }

    pub fn remove_all_listeners(&mut self) {
        self.listeners.clear();
        self.once_listeners.clear();
    }

    pub fn listener_count(&self, event: &str) -> usize {
        self.listeners.get(event).map_or(0, |v| v.len())
            + self.once_listeners.get(event).map_or(0, |v| v.len())
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}
