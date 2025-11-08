pub mod event_list;
use std::{
  any::Any,
  sync::{Arc, LazyLock, Mutex},
};

type EvtType = LazyLock<Mutex<EventSystem<Box<dyn Fn(Option<&dyn Any>) + Send>>>>;
pub type EventSystemType = Arc<Mutex<EventSystem<Box<dyn Fn(Option<&dyn Any>) + Send>>>>;

/// The global event system
static EVENT_SYSTEM: EvtType = LazyLock::new(|| Mutex::new(EventSystem::new()));

pub struct EventSystem<T> {
  events: Vec<Event<T>>,
}

/// Adds an event to the global event system
pub fn add_listener<T>(name: &str, action: T)
where
  T: Fn(Option<&dyn Any>) + Send + 'static,
{
  let mut event_system = EVENT_SYSTEM.lock().unwrap();
  event_system.add_listener(name, Box::new(action));
}

/// Adds an event to the specified event system
pub fn add_listener_to<T>(event_system: &EventSystemType, name: &str, action: T)
where
  T: Fn(Option<&dyn Any>) + Send + 'static,
{
  let mut event_system = event_system.lock().unwrap();
  event_system.add_listener(name, Box::new(action));
}

/// Dispatches an event on the global event system
pub fn dispatch(name: &str, data: Option<&dyn Any>) {
  let event_system = EVENT_SYSTEM.lock().unwrap();
  event_system.dispatch(name, data);
}

/// Dispatches an event on the specified event system
pub fn dispatch_to(event_system: &EventSystemType, name: &str, data: Option<&dyn Any>) {
  let event_system = event_system.lock().unwrap();
  event_system.dispatch(name, data);
}

#[derive(Clone)]
pub struct Event<T> {
  pub name: String,
  pub action: Box<T>,
}

impl<T> EventSystem<T>
where
  T: Fn(Option<&dyn Any>) + Send + 'static,
{
  /// Creates a new event system
  pub fn new() -> Self {
    Self { events: vec![] }
  }

  /// Adds an event to the global event system
  pub fn add_listener(&mut self, name: &str, action: T) {
    let event = Event {
      name: name.to_string(),
      action: Box::new(action),
    };
    self.events.push(event);
  }

  /// Dispatches an event on the global event system
  pub fn dispatch(&self, name: &str, data: Option<&dyn Any>) {
    for event in &self.events {
      if event.name == name {
        (event.action)(data);
      }
    }
  }
}
