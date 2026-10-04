//! Vessel: a toolkit for building apps out of components.
//!
//! This crate is the one to depend on. It brings together the parts, and `use vessel::prelude::*;` is the one import an app
//! needs:
//!
//! - the **engine**: an always-on loop that decides when to draw (`Engine`, `View`, `MediaSource`, `Frame`, `GpuFrame`);
//! - the **API**: `Component` with typed events as streams, `BehaviorSubject`, `Canvas`, layout, theme, fonts, input, `App` and
//!   `Session`;
//! - ready-made **controls**: `Button`, `Label`, `Slider` and `Image`.
//!
//! Where an app shows its pictures is chosen with cargo features, none on by default: `desktop-window` for a window Vessel
//! owns, `android-surface` for a view that another application owns.
//!
//! ```ignore
//! use vessel::prelude::*;
//!
//! let save = Button::new("Save").with_primary(true);
//! save.subject::<Clicked>().subscribe(|_| println!("saved"));
//! App::new("Notes").add(&save).run().unwrap();
//! ```
#![deny(missing_docs)]

// The parts by name (`vessel::ui::Button`), and everything at the top (`vessel::Button`).
pub use vessel_api as api;
pub use vessel_api::*;
pub use vessel_engine as engine;
pub use vessel_engine::*;
pub use vessel_ui as ui;
pub use vessel_macros::Component;
pub use vessel_ui::*;

/// What an app usually needs, in one import: the engine's items, the API, the controls, and the `pub-sub` traits that
/// `subscribe` and `next` are methods of.
pub mod prelude {
  pub use vessel_api::prelude::*;
  pub use vessel_macros::Component;
  pub use vessel_ui::{Button, Changed, Clicked, Container, Fit, Image, Label, Picture, Slider, Trackable};
}
