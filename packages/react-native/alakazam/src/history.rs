use std::sync::{Arc, Mutex};

use abra::prelude::*;

use crate::AbraImage;

#[derive(Default)]
struct HistoryState {
  entries: Vec<Image>,
  index: usize,
}

/// A cursor-based sequence of complete Rust-owned image states.
///
/// Pushing after Undo removes the redo branch. No pixel data crosses the JavaScript bridge: callers
/// exchange only `AbraImage` handles.
#[derive(uniffi::Object)]
pub struct AbraImageHistory {
  state: Mutex<HistoryState>,
}

#[uniffi::export]
impl AbraImageHistory {
  /// Starts a history with `image` as its first state.
  #[uniffi::constructor]
  pub fn new(image: Arc<AbraImage>) -> Arc<Self> {
    Arc::new(Self {
      state: Mutex::new(HistoryState {
        entries: vec![image.clone_image()],
        index: 0,
      }),
    })
  }

  /// Adds an image state, discarding any redo branch after the current cursor.
  pub fn push(&self, image: Arc<AbraImage>) {
    let mut state = self.state.lock().unwrap();
    let next_entry = state.index + 1;
    state.entries.truncate(next_entry);
    state.entries.push(image.clone_image());
    state.index = state.entries.len() - 1;
  }

  pub fn can_undo(&self) -> bool {
    self.state.lock().unwrap().index > 0
  }

  pub fn can_redo(&self) -> bool {
    let state = self.state.lock().unwrap();
    state.index + 1 < state.entries.len()
  }

  /// Moves to and returns the previous image state, if any.
  pub fn undo(&self) -> Option<Arc<AbraImage>> {
    let image = {
      let mut state = self.state.lock().unwrap();
      if state.index == 0 {
        return None;
      }
      state.index -= 1;
      state.entries[state.index].clone()
    };
    Some(AbraImage::from_image(image))
  }

  /// Moves to and returns the next image state, if any.
  pub fn redo(&self) -> Option<Arc<AbraImage>> {
    let image = {
      let mut state = self.state.lock().unwrap();
      if state.index + 1 >= state.entries.len() {
        return None;
      }
      state.index += 1;
      state.entries[state.index].clone()
    };
    Some(AbraImage::from_image(image))
  }

  /// Starts a new history rooted at `image`.
  pub fn reset(&self, image: Arc<AbraImage>) {
    let mut state = self.state.lock().unwrap();
    state.entries.clear();
    state.entries.push(image.clone_image());
    state.index = 0;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra::prelude::Channels;

  #[test]
  fn undo_and_redo_return_complete_image_states() {
    let first = AbraImage::from_image(Image::new_from_pixels(1, 1, vec![10, 20, 30, 255], Channels::RGBA));
    let history = AbraImageHistory::new(first.clone());
    let second = AbraImage::from_image(Image::new_from_pixels(1, 1, vec![40, 50, 60, 255], Channels::RGBA));
    history.push(second.clone());

    assert_eq!(history.undo().unwrap().rgba(), first.rgba());
    assert_eq!(history.redo().unwrap().rgba(), second.rgba());
  }
}
