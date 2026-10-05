use vessel_api::prelude::*;
use vessel_macros::Component;

use crate::{UiBackground, background::BackgroundLayer};

/// Holds other components and places them, like a CSS `div` with `display: flex` or `grid`. Stacked is the default: every
/// child at the top left, later ones in front. `with_display` changes that, and the other settings say how:
///
/// - **Flex** puts children in a line along `direction` (a row or a column), spaced by `gap`, moved to the next line when
///   `wrap` is on, and placed along the line with `justify` and across it with `align`.
/// - **Grid** puts children in cells given by `columns` and `rows`; a child can `span` several.
///
/// Like any component it has the box settings (`set_padding`, `set_margin`, `set_border_width`, `set_radius`,
/// `set_background`) and a size, and it goes in a parent with `add`, so containers nest.
/// Use `with_width(Units::Percent(50.0))` or `with_height(Units::Pixels(100))` to size it inside its parent; plain numbers
/// still mean pixels. The inherited `set_width` and `set_height` change these sizes later.
/// `with_background(Background::Surface)` sets a theme/color fill; `with_background(&image)` paints an image behind
/// content, outside child layout and input. Its `Fit` controls sizing (`Contain`, `Cover`, or `Stretch`), centered,
/// without repetition. Padding does not shrink the background, and rounded corners clip it. GPU backgrounds are
/// read back for CPU composition. `set_background` replaces the background; `Background::None` clears it.
///
/// ```
/// use vessel_ui::{Button, Container, Slider};
/// use vessel_api::prelude::*;
///
/// let slider = Slider::new(0.5);
/// let toolbar = Container::new().with_display(Display::Flex).with_gap(8).with_padding(12);
/// toolbar.add(Button::new("Save")).add(&slider);
/// let page = Container::new().with_background(Background::Surface).add(&toolbar).with_gap(8);
/// ```
#[derive(Clone, Component)]
#[component(plain)]
pub struct Container {
  component: Component,
  background: BackgroundLayer,
}

impl Container {
  component_builders!();

  /// A container that stacks its children. Call `with_display` to place them another way.
  pub fn new() -> Self {
    let component = Component::new("container");
    let background = BackgroundLayer::new(&component);
    Self { component, background }
  }

  /// Replaces the theme/color or image background. An image paints behind content and does not receive input.
  /// Use [`Background::None`] to clear it. An unreadable GPU background panics rather than silently disappearing.
  pub fn set_background(&self, p_background: impl Into<UiBackground>) {
    self.background.set(&self.component, p_background.into());
  }

  /// Sets a background and returns the container for chaining. See [`set_background`](Self::set_background).
  pub fn with_background(&self, p_background: impl Into<UiBackground>) -> Self {
    self.set_background(p_background);
    self.clone()
  }
}

impl Default for Container {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;

  /// A component that fills itself with one color.
  fn swatch(p_color: [u8; 4]) -> Component {
    let component = Component::new("swatch");
    component.subject::<Canvas>().subscribe(move |canvas| canvas.fill(p_color));
    component
  }

  fn render(p_container: &Container) -> Frame {
    let mut component = (**p_container).clone();
    component.render(Duration::ZERO)
  }

  fn pixel(p_frame: &Frame, p_x: u32, p_y: u32) -> [u8; 4] {
    let at = ((p_y * p_frame.width + p_x) * 4) as usize;
    p_frame.pixels[at..at + 4].try_into().unwrap()
  }

  const RED: [u8; 4] = [200, 0, 0, 255];
  const BLUE: [u8; 4] = [0, 0, 200, 255];

  #[test]
  fn borrowed_controls_and_nested_containers_can_be_added_without_losing_the_container_type() {
    let image = crate::Image::new();
    let label = crate::Label::new("hello");
    let button = crate::Button::new("save");
    let slider = crate::Slider::new(0.5);
    let inner = Container::new().add(&image).add(&label).add(&button).add(&slider).with_display(Display::Flex);
    let outer = Container::new().add(&inner).with_direction(Direction::Column);
    outer.size().next((40, 10));
    render(&outer);
    assert_eq!(inner.size().value(), (40, 10));
    for child in [&*image, &*label, &*button, &*slider] {
      assert_eq!(child.size().value(), (10, 10));
    }
  }

  #[test]
  fn component_parents_accept_borrowed_controls_and_smart_pointers() {
    let inner = Container::new();
    inner.add(swatch(RED));
    let shared = std::sync::Arc::new(Container::new());
    shared.add(swatch(BLUE));
    let parent = Component::new("parent").with_display(Display::Flex).with_size(4, 1);
    parent.add(&inner).add(&shared);
    let frame = parent.clone().render(Duration::ZERO);
    assert_eq!(pixel(&frame, 0, 0), RED);
    assert_eq!(pixel(&frame, 3, 0), BLUE);
  }

  #[test]
  fn it_stacks_its_children_until_told_otherwise_with_the_later_one_in_front() {
    let container = Container::new();
    container.size().next((4, 1));
    container.add(swatch(RED).fill()).add(swatch(BLUE).width(1));
    let frame = render(&container);
    assert_eq!(pixel(&frame, 0, 0), BLUE, "the later child is in front");
    assert_eq!(pixel(&frame, 3, 0), RED);
  }

  #[test]
  fn flex_places_children_side_by_side_along_a_row_and_one_over_the_other_down_a_column() {
    let row = Container::new().with_display(Display::Flex).with_direction(Direction::Row);
    row.size().next((4, 2));
    row.add(swatch(RED).fill()).add(swatch(BLUE).fill());
    let frame = render(&row);
    assert_eq!((pixel(&frame, 0, 0), pixel(&frame, 3, 0)), (RED, BLUE));

    let column = Container::new().with_display(Display::Flex).with_direction(Direction::Column);
    column.size().next((2, 4));
    column.add(swatch(RED).fill()).add(swatch(BLUE).fill());
    let frame = render(&column);
    assert_eq!((pixel(&frame, 0, 0), pixel(&frame, 0, 3)), (RED, BLUE));
  }

  #[test]
  fn a_grid_container_puts_children_in_its_columns() {
    let grid =
      Container::new().with_display(Display::Grid).with_columns([Track::Sized(Units::Pixels(1)), Track::Fr(1.0)]);
    grid.size().next((4, 1));
    grid.add(swatch(RED).fill()).add(swatch(BLUE).fill());
    let frame = render(&grid);
    assert_eq!((pixel(&frame, 0, 0), pixel(&frame, 1, 0), pixel(&frame, 3, 0)), (RED, BLUE, BLUE));
  }

  #[test]
  fn unit_tracks_follow_resizing_and_reactive_column_and_row_setters() {
    let container = Container::new()
      .with_display(Display::Grid)
      .with_columns([Track::Sized(Units::Percent(25.0)), Track::Fr(1.0)])
      .with_rows([Track::Sized(Units::Percent(50.0))]);
    let first = swatch(RED);
    let second = swatch(BLUE);
    container.add(&first).add(&second);
    container.size().next((8, 4));
    let frame = render(&container);
    assert_eq!(first.size().value(), (2, 2));
    assert_eq!(second.size().value(), (6, 2));
    assert_eq!(pixel(&frame, 1, 1), RED);
    assert_eq!(pixel(&frame, 2, 1), BLUE);
    container.size().next((16, 8));
    render(&container);
    assert_eq!(first.size().value(), (4, 4));
    assert_eq!(second.size().value(), (12, 4));
    container.set_columns([Track::Sized(Units::Pixels(3)), Track::Fr(1.0)]);
    container.set_rows([Track::Sized(Units::Percent(25.0))]);
    render(&container);
    assert_eq!(first.size().value(), (3, 2));
    assert_eq!(second.size().value(), (13, 2));
  }

  #[test]
  fn a_container_goes_inside_another_and_gap_and_padding_make_room() {
    let inner = Container::new().with_display(Display::Flex).with_gap(2);
    inner.add(swatch(RED).width(1).fill()).add(swatch(BLUE).fill());
    let outer = Container::new();
    outer.set_padding(1);
    outer.size().next((8, 3));
    outer.add(inner.fill());
    let frame = render(&outer);
    assert_eq!(pixel(&frame, 0, 0), [0, 0, 0, 0], "the padding is left clear");
    assert_ne!(pixel(&frame, 1, 1), [0, 0, 0, 0], "the inner container is drawn inside it");
  }

  #[test]
  fn numeric_size_builders_default_to_pixels() {
    let inner = Container::new().with_width(3).with_height(2);
    let outer = Container::new();
    outer.size().next((10, 10));
    outer.add(inner.clone());
    render(&outer);
    assert_eq!(inner.size().value(), (3, 2));
  }

  #[test]
  fn unit_sizes_follow_parent_resizing_and_reactive_setters() {
    let inner =
      Container::new().with_width(Units::Percent(50.0)).with_height(Units::Pixels(2)).with_display(Display::Flex);
    inner.add(swatch(RED));
    let outer = Container::new();
    outer.size().next((8, 4));
    outer.add(inner.clone());
    let frame = render(&outer);
    assert_eq!(inner.size().value(), (4, 2));
    assert_eq!(pixel(&frame, 3, 1), RED);
    assert_eq!(pixel(&frame, 4, 1), [0, 0, 0, 0]);
    assert_eq!(pixel(&frame, 3, 2), [0, 0, 0, 0]);

    outer.size().next((12, 6));
    render(&outer);
    assert_eq!(inner.size().value(), (6, 2));

    inner.set_width(Units::Pixels(3));
    inner.set_height(Units::Percent(50.0));
    let frame = render(&outer);
    assert_eq!(inner.size().value(), (3, 3));
    assert_eq!(pixel(&frame, 2, 2), RED);
    assert_eq!(pixel(&frame, 3, 2), [0, 0, 0, 0]);

    inner.set_width(5);
    inner.set_height(1);
    render(&outer);
    assert_eq!(inner.size().value(), (5, 1));
  }
}
