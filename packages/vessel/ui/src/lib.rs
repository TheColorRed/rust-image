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
//! Shared style and layout builders return the concrete control, just like its specialized builders. Setters remain
//! available through its component. Common styles and control-specific settings can be chained in any order:
//!
//! ```
//! use vessel_api::prelude::*;
//! use vessel_ui::{Button, Container};
//!
//! let button = Button::new("Save").with_padding(8).with_label("Open").with_radius(4);
//! let page = Container::new().with_padding(12).with_background(Background::Surface).add(&button).with_gap(8);
//! button.set_label("Save");
//! page.set_padding(16);
//! ```
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

macro_rules! component_builders {
  () => {
    component_builders! {
      with_size(p_width: u32, p_height: u32);
      with_display(p_display: Display);
      with_direction(p_direction: Direction);
      with_wrap(p_wrap: bool);
      with_gap(p_gap: u32);
      with_padding(p_padding: impl Into<Edges>);
      with_margin(p_margin: impl Into<Edges>);
      with_border_width(p_width: u32);
      with_border_color(p_color: [u8; 4]);
      with_radius(p_radius: impl Into<Units>);
      with_justify(p_justify: Justify);
      with_align(p_align: Align);
      with_columns(p_columns: impl IntoIterator<Item = Track>);
      with_rows(p_rows: impl IntoIterator<Item = Track>);
      with_font_size(p_size: impl Into<Units>);
      with_font_face(p_face: Face);
      with_text_align(p_align: TextAlign);
      with_theme(p_theme: impl Into<ThemePatch>);
      with_width(p_width: impl Into<Units>);
      with_height(p_height: impl Into<Units>);
      width(p_width: impl Into<Units>);
      height(p_height: impl Into<Units>);
      fill();
      grow(p_share: f32);
      span(p_columns: u16, p_rows: u16);
      add(p_child: impl Renderable);
    }
  };
  ($($method:ident($($arg:ident: $type:ty),*);)*) => {
    $(
      #[doc = concat!("Applies [`Component::", stringify!($method), "`](vessel_api::Component::", stringify!($method),
        ") and returns this control, preserving its specialized methods.")]
      pub fn $method(&self, $($arg: $type),*) -> Self {
        self.component.$method($($arg),*);
        self.clone()
      }
    )*
  };
}

macro_rules! color_background_builder {
  () => {
    /// Sets the theme/color background and returns this control, preserving its specialized methods.
    pub fn with_background(&self, p_background: Background) -> Self {
      self.set_background(p_background);
      self.clone()
    }
  };
}

mod background;
mod button;
mod container;
mod image;
mod label;
mod slider;

pub use self::{
  background::UiBackground,
  button::{Button, Clicked},
  container::Container,
  image::{Fit, Image, Picture, Trackable},
  label::{Label, VerticalAlign},
  slider::{Changed, Slider},
};

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use vessel_api::prelude::*;

  use super::*;

  macro_rules! styled {
    ($control:expr) => {
      $control
        .with_size(40, 30)
        .with_display(Display::Grid)
        .with_direction(Direction::Column)
        .with_wrap(true)
        .with_gap(2)
        .with_padding([2, 3])
        .with_margin([1, 2, 3, 4])
        .with_border_width(1)
        .with_border_color([20, 30, 40, 255])
        .with_radius(0)
        .with_justify(Justify::Center)
        .with_align(Align::End)
        .with_columns([Track::Fr(1.0)])
        .with_rows([Track::Fr(1.0)])
        .with_font_size(Units::Pixels(12))
        .with_font_size(Units::Em(1.25))
        .with_font_size(Units::Percent(75.0))
        .with_font_face(Face::bitmap())
        .with_text_align(TextAlign::Center)
        .with_theme(Theme::dark())
        .with_width(Units::Percent(50.0))
        .with_height(20)
        .width(10)
        .height(10)
        .fill()
        .grow(2.0)
        .span(1, 1)
        .add(Component::new("child"))
        .with_background(Background::Surface)
    };
  }

  #[test]
  fn every_shared_builder_preserves_each_controls_type_and_specific_methods() {
    let container: Container = styled!(Container::new());
    let button: Button = styled!(Button::new("before")).with_label("after").with_primary(true);
    let label: Label = styled!(Label::new("before")).with_text("after").with_color([255; 4]);
    let image: Image = styled!(Image::new()).with_fit(Fit::Cover).with_pixels(1, 1, vec![255; 4]);
    let slider: Slider = styled!(Slider::new(0.0)).with_value(0.75).with_color([255; 4]);
    assert_eq!(slider.value(), 0.75);
    for control in [&*container, &*button, &*label, &*image, &*slider] {
      assert_eq!(control.size().value(), (40, 30));
      assert_eq!(control.font().size, 12);
      assert_eq!(control.theme(), Theme::dark());
    }
  }

  #[test]
  fn shared_styles_render_and_inherited_setters_remain_reactive() {
    let container = Container::new()
      .with_size(12, 10)
      .with_padding(2)
      .with_border_width(1)
      .with_border_color([0, 0, 200, 255])
      .with_background(Background::Color([200, 0, 0, 255]))
      .with_radius(0)
      .with_display(Display::Flex);
    let child = Label::new("").with_radius(0).with_background(Background::Color([0, 200, 0, 255])).with_text(" ");
    container.add(&child);
    let frame = container.component().clone().render(Duration::ZERO);
    assert_eq!(child.size().value(), (6, 4));
    assert_eq!(&frame.pixels.as_slice()[0..4], &[0, 0, 200, 255]);
    let pixel = |x: u32, y: u32| {
      let at = ((y * frame.width + x) * 4) as usize;
      &frame.pixels.as_slice()[at..at + 4]
    };
    assert_eq!(pixel(1, 1), &[200, 0, 0, 255]);
    assert_eq!(pixel(3, 3), &[0, 200, 0, 255]);

    container.set_padding(1);
    container.component().clone().render(Duration::ZERO);
    assert_eq!(child.size().value(), (8, 6));
    container.set_font_size(18);
    assert_eq!(child.font().size, 18);
    container.set_columns([Track::Sized(Units::Pixels(4)), Track::Fr(1.0)]);
    container.set_rows([Track::Fr(1.0)]);
    container.set_display(Display::Grid);
    container.component().clone().render(Duration::ZERO);
    assert_eq!(child.size().value(), (4, 6));
  }
}
