use napi::bindgen_prelude::Reference;

use crate::common::*;

#[napi(js_name = "AbraHistoryEntry")]
#[derive(Clone)]
pub struct HistoryEntry {
  description: String,
  timestamp: u32,
  data: ImageData,
}

#[napi]
impl HistoryEntry {
  #[napi(constructor)]
  pub fn new(description: String, data: ImageData) -> Self {
    Self {
      description,
      timestamp: Self::current_timestamp(),
      data,
    }
  }

  #[napi(getter)]
  pub fn description(&self) -> String {
    self.description.clone()
  }

  #[napi(getter)]
  pub fn timestamp(&self) -> u32 {
    self.timestamp
  }

  #[napi(getter)]
  pub fn data(&self) -> ImageData {
    self.data.clone()
  }

  fn current_timestamp() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
    since_epoch.as_secs() as u32
  }
}

#[napi(js_name = "AbraHistory")]
pub struct History {
  project_id: String,
  entries: Vec<HistoryEntry>,
}

#[napi]
impl History {
  #[napi(constructor)]
  pub fn new(project_id: String) -> Self {
    Self {
      project_id,
      entries: Vec::new(),
    }
  }

  #[napi(getter)]
  /// Get the number of entries in the history.
  pub fn length(&self) -> u32 {
    self.entries.len() as u32
  }

  #[napi(getter)]
  /// Get the project ID associated with this history.
  pub fn project_id(&self) -> String {
    self.project_id.clone()
  }

  #[napi]
  /// Add a new entry to the end of the history.
  /// - `item`: The history entry to add.
  pub fn add(&mut self, item: Reference<HistoryEntry>) {
    self.entries.push((*item).clone());
  }

  #[napi]
  /// Remove an entry at the specified index.
  /// @param index The index of the entry to remove.
  /// @returns true if the entry was removed, false if the index was out of bounds
  pub fn remove(&mut self, index: u32) -> bool {
    let index = index as usize;
    if index < self.entries.len() {
      self.entries.remove(index);
      true
    } else {
      false
    }
  }

  #[napi]
  /// Remove the last entry in the history.
  /// @returns true if an entry was removed, false if the history is empty
  pub fn remove_last(&mut self) -> bool {
    let index = self.entries.len().saturating_sub(1);
    self.remove(index as u32)
  }

  #[napi]
  pub fn get(&self, index: u32) -> Option<HistoryEntry> {
    self.entries.get(index as usize).cloned()
  }
}
