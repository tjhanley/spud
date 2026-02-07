use std::collections::HashMap;

use anyhow::{bail, Result};

use crate::event::Event;
use crate::module::Module;

/// Owns and manages all registered SPUD modules.
///
/// Modules are stored in insertion order and indexed by their unique
/// [`Module::id`]. The registry tracks which module is currently active and
/// provides cycling, activation, and event broadcasting.
pub struct ModuleRegistry {
    modules: Vec<Box<dyn Module>>,
    active_idx: Option<usize>,
    index: HashMap<String, usize>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {
    /// Create an empty registry with no modules.
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            active_idx: None,
            index: HashMap::new(),
        }
    }

    /// Register a module. The first module registered is automatically activated.
    ///
    /// # Errors
    ///
    /// Returns an error if a module with the same ID is already registered.
    pub fn register(&mut self, module: Box<dyn Module>) -> Result<()> {
        let id = module.id().to_string();
        if self.index.contains_key(&id) {
            bail!("duplicate module id: {}", id);
        }
        let idx = self.modules.len();
        self.index.insert(id, idx);
        self.modules.push(module);
        if self.active_idx.is_none() {
            self.active_idx = Some(0);
        }
        Ok(())
    }

    /// Activate a module by ID, returning lifecycle events.
    ///
    /// Returns a `ModuleDeactivated` event for the previously active module
    /// (if different) followed by a `ModuleActivated` event for the new one.
    ///
    /// # Errors
    ///
    /// Returns an error if no module with the given ID exists.
    pub fn activate(&mut self, id: &str) -> Result<Vec<Event>> {
        let idx = self.index.get(id).copied();
        match idx {
            Some(i) => {
                let mut events = Vec::new();
                if let Some(old) = self.active_idx {
                    if old != i {
                        events.push(Event::ModuleDeactivated {
                            id: self.modules[old].id().to_string(),
                        });
                    }
                }
                self.active_idx = Some(i);
                events.push(Event::ModuleActivated { id: id.to_string() });
                Ok(events)
            }
            None => bail!("unknown module id: {}", id),
        }
    }

    /// Return a reference to the currently active module, or `None` if empty.
    pub fn active(&self) -> Option<&dyn Module> {
        self.active_idx.map(|i| &*self.modules[i])
    }

    /// Return a mutable reference to the currently active module.
    pub fn active_mut(&mut self) -> Option<&mut (dyn Module + 'static)> {
        self.active_idx.map(|i| &mut *self.modules[i])
    }

    /// Return the ID of the currently active module, or `None` if empty.
    pub fn active_id(&self) -> Option<&str> {
        self.active_idx.map(|i| self.modules[i].id())
    }

    /// Cycle to the next module (wrapping around), returning lifecycle events.
    pub fn cycle_next(&mut self) -> Vec<Event> {
        if self.modules.is_empty() {
            return Vec::new();
        }
        let cur = self.active_idx.unwrap_or(0);
        let next = (cur + 1) % self.modules.len();
        self.switch_to(cur, next)
    }

    /// Cycle to the previous module (wrapping around), returning lifecycle events.
    pub fn cycle_prev(&mut self) -> Vec<Event> {
        if self.modules.is_empty() {
            return Vec::new();
        }
        let cur = self.active_idx.unwrap_or(0);
        let next = if cur == 0 {
            self.modules.len() - 1
        } else {
            cur - 1
        };
        self.switch_to(cur, next)
    }

    /// Internal helper to switch between two module indices.
    fn switch_to(&mut self, from: usize, to: usize) -> Vec<Event> {
        let mut events = Vec::new();
        if from != to {
            events.push(Event::ModuleDeactivated {
                id: self.modules[from].id().to_string(),
            });
        }
        self.active_idx = Some(to);
        events.push(Event::ModuleActivated {
            id: self.modules[to].id().to_string(),
        });
        events
    }

    /// Return `(id, title)` pairs for all registered modules in order.
    pub fn list(&self) -> Vec<(&str, &str)> {
        self.modules.iter().map(|m| (m.id(), m.title())).collect()
    }

    /// Look up a module by ID.
    pub fn get(&self, id: &str) -> Option<&dyn Module> {
        self.index.get(id).map(|&i| &*self.modules[i])
    }

    /// Look up a module by ID (mutable).
    pub fn get_mut(&mut self, id: &str) -> Option<&mut (dyn Module + 'static)> {
        self.index.get(id).copied().map(|i| &mut *self.modules[i])
    }

    /// Return the number of registered modules.
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Return `true` if no modules are registered.
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Broadcast an event to modules.
    ///
    /// Routing rules:
    /// - `Tick` and `Resize` — sent to **all** modules.
    /// - `Key` — sent to the **active** module only.
    /// - `ModuleActivated` / `ModuleDeactivated` — sent to the **named** module.
    /// - Everything else (`Telemetry`, `Custom`, `Quit`) — sent to **all** modules.
    pub fn broadcast(&mut self, event: &Event) {
        match event {
            Event::Tick { .. } | Event::Resize { .. } => {
                for m in &mut self.modules {
                    m.handle_event(event);
                }
            }
            Event::Key(_) => {
                if let Some(m) = self.active_mut() {
                    m.handle_event(event);
                }
            }
            Event::ModuleActivated { id } => {
                if let Some(idx) = self.index.get(id).copied() {
                    self.modules[idx].handle_event(event);
                }
            }
            Event::ModuleDeactivated { id } => {
                if let Some(idx) = self.index.get(id).copied() {
                    self.modules[idx].handle_event(event);
                }
            }
            _ => {
                for m in &mut self.modules {
                    m.handle_event(event);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    struct FakeModule {
        id: &'static str,
        title: &'static str,
        events: Arc<Mutex<Vec<String>>>,
    }

    impl FakeModule {
        fn new(id: &'static str, title: &'static str) -> Self {
            Self {
                id,
                title,
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn with_log(id: &'static str, title: &'static str, log: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                id,
                title,
                events: log,
            }
        }
    }

    impl Module for FakeModule {
        fn id(&self) -> &'static str {
            self.id
        }
        fn title(&self) -> &'static str {
            self.title
        }
        fn handle_event(&mut self, ev: &Event) {
            let tag = match ev {
                Event::Tick { .. } => "tick",
                Event::Key(_) => "key",
                Event::Resize { .. } => "resize",
                Event::ModuleActivated { .. } => "activated",
                Event::ModuleDeactivated { .. } => "deactivated",
                Event::Quit => "quit",
                _ => "other",
            };
            self.events
                .lock()
                .unwrap()
                .push(format!("{}:{}", self.id, tag));
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn register_adds_module() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::new("a", "Alpha")))
            .unwrap();
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.list(), vec![("a", "Alpha")]);
    }

    #[test]
    fn duplicate_id_returns_error() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::new("a", "Alpha")))
            .unwrap();
        let err = reg.register(Box::new(FakeModule::new("a", "Alpha2")));
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("duplicate module id"));
    }

    #[test]
    fn activate_by_id() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::new("a", "Alpha")))
            .unwrap();
        reg.register(Box::new(FakeModule::new("b", "Beta")))
            .unwrap();
        reg.activate("b").unwrap();
        assert_eq!(reg.active().unwrap().id(), "b");
    }

    #[test]
    fn activate_invalid_id_returns_error() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::new("a", "Alpha")))
            .unwrap();
        let err = reg.activate("nope");
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("unknown module id"));
    }

    #[test]
    fn activate_emits_lifecycle_events() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::new("a", "Alpha")))
            .unwrap();
        reg.register(Box::new(FakeModule::new("b", "Beta")))
            .unwrap();
        let events = reg.activate("b").unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], Event::ModuleDeactivated { id } if id == "a"));
        assert!(matches!(&events[1], Event::ModuleActivated { id } if id == "b"));
    }

    #[test]
    fn first_register_auto_activates() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::new("a", "Alpha")))
            .unwrap();
        assert_eq!(reg.active().unwrap().id(), "a");
    }

    #[test]
    fn empty_registry_returns_none() {
        let reg = ModuleRegistry::new();
        assert!(reg.active().is_none());
        assert!(reg.active_id().is_none());
    }

    #[test]
    fn cycle_next_wraps() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::new("a", "Alpha")))
            .unwrap();
        reg.register(Box::new(FakeModule::new("b", "Beta")))
            .unwrap();
        reg.register(Box::new(FakeModule::new("c", "Gamma")))
            .unwrap();

        assert_eq!(reg.active_id(), Some("a"));
        reg.cycle_next();
        assert_eq!(reg.active_id(), Some("b"));
        reg.cycle_next();
        assert_eq!(reg.active_id(), Some("c"));
        reg.cycle_next();
        assert_eq!(reg.active_id(), Some("a"));
    }

    #[test]
    fn cycle_next_emits_lifecycle() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::new("a", "Alpha")))
            .unwrap();
        reg.register(Box::new(FakeModule::new("b", "Beta")))
            .unwrap();
        let events = reg.cycle_next();
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], Event::ModuleDeactivated { id } if id == "a"));
        assert!(matches!(&events[1], Event::ModuleActivated { id } if id == "b"));
    }

    #[test]
    fn cycle_prev_wraps() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::new("a", "Alpha")))
            .unwrap();
        reg.register(Box::new(FakeModule::new("b", "Beta")))
            .unwrap();
        reg.register(Box::new(FakeModule::new("c", "Gamma")))
            .unwrap();

        assert_eq!(reg.active_id(), Some("a"));
        reg.cycle_prev();
        assert_eq!(reg.active_id(), Some("c"));
        reg.cycle_prev();
        assert_eq!(reg.active_id(), Some("b"));
    }

    #[test]
    fn list_returns_id_title_pairs() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::new("x", "X-Ray")))
            .unwrap();
        reg.register(Box::new(FakeModule::new("y", "Yankee")))
            .unwrap();
        assert_eq!(reg.list(), vec![("x", "X-Ray"), ("y", "Yankee")]);
    }

    #[test]
    fn cycle_on_empty_is_noop() {
        let mut reg = ModuleRegistry::new();
        reg.cycle_next();
        reg.cycle_prev();
        assert!(reg.active().is_none());
    }

    #[test]
    fn get_and_get_mut_by_id() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::new("a", "Alpha")))
            .unwrap();
        assert_eq!(reg.get("a").unwrap().title(), "Alpha");
        assert!(reg.get("z").is_none());
        assert!(reg.get_mut("a").is_some());
        assert!(reg.get_mut("z").is_none());
    }

    #[test]
    fn broadcast_tick_goes_to_all() {
        let log_a = Arc::new(Mutex::new(Vec::new()));
        let log_b = Arc::new(Mutex::new(Vec::new()));
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::with_log("a", "Alpha", log_a.clone())))
            .unwrap();
        reg.register(Box::new(FakeModule::with_log("b", "Beta", log_b.clone())))
            .unwrap();

        reg.broadcast(&Event::Tick {
            now: Instant::now(),
        });
        assert_eq!(log_a.lock().unwrap().as_slice(), &["a:tick"]);
        assert_eq!(log_b.lock().unwrap().as_slice(), &["b:tick"]);
    }

    #[test]
    fn broadcast_key_goes_to_active_only() {
        let log_a = Arc::new(Mutex::new(Vec::new()));
        let log_b = Arc::new(Mutex::new(Vec::new()));
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::with_log("a", "Alpha", log_a.clone())))
            .unwrap();
        reg.register(Box::new(FakeModule::with_log("b", "Beta", log_b.clone())))
            .unwrap();

        let key = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('x'),
            crossterm::event::KeyModifiers::NONE,
        );
        reg.broadcast(&Event::Key(key));
        assert_eq!(log_a.lock().unwrap().as_slice(), &["a:key"]);
        assert!(log_b.lock().unwrap().is_empty());
    }

    #[test]
    fn broadcast_lifecycle_goes_to_target() {
        let log_a = Arc::new(Mutex::new(Vec::new()));
        let log_b = Arc::new(Mutex::new(Vec::new()));
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(FakeModule::with_log("a", "Alpha", log_a.clone())))
            .unwrap();
        reg.register(Box::new(FakeModule::with_log("b", "Beta", log_b.clone())))
            .unwrap();

        reg.broadcast(&Event::ModuleActivated { id: "b".into() });
        assert!(log_a.lock().unwrap().is_empty());
        assert_eq!(log_b.lock().unwrap().as_slice(), &["b:activated"]);
    }
}
