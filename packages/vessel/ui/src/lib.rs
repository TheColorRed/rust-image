//! Reusable components for Vessel apps: [`Button`], [`Label`], [`Slider`], [`Image`] and [`Container`].
//!
//! Each one is a struct that keeps a `vessel_api::Component` inside and dereferences to it, so it can be added to a parent
//! like any component and has the same events and layout methods. Its look follows its own reactive properties:
//! `button.set_label("Close")` redraws it, and the user never creates a stream. Things the user does come out as typed
//! events on the component, which you listen to with `subscribe`:
//!
//! ```ignore
//! use vessel_api::prelude::*;
//! use vessel_ui::{Button, Clicked};
//!
//! let save = Button::new("Save").with_color([64, 120, 220, 255]);
//! save.subject::<Clicked>().subscribe(|_| println!("saved"));
//! page.add(save.width(120));
//! ```
//!
//! These components use no abra and no platform code, so they run in any Vessel host.
#![deny(missing_docs)]

/// Makes a `set_*` and a `with_*` for each property of a component struct. `with_*` is for chaining while building.
macro_rules! properties {
  ($($field:ident: $arg:ty => $set:ident, $with:ident;)*) => {
    $(
      #[doc = concat!("Sets the `", stringify!($field), "` and redraws.")]
      pub fn $set(&self, p_value: $arg) {
        self.$field.next(p_value.into());
      }

      #[doc = concat!("Sets the `", stringify!($field), "`, and returns the component so calls can be chained.")]
      pub fn $with(self, p_value: $arg) -> Self {
        self.$set(p_value);
        self
      }
    )*
  };
}

mod button;
mod container;
mod image;
mod label;
mod slider;

pub use self::{
  button::{Button, Clicked},
  container::Container,
  image::{Fit, Image, Picture, Trackable},
  label::Label,
  slider::{Changed, Slider},
};
