
use vessel_api::prelude::*;
use vessel_macros::Component;

/// Makes the `set_*` and `with_*` pair of a layout setting that the container passes on to its component.
macro_rules! layout {
  ($($set:ident, $with:ident: $arg:ty;)*) => {
    $(
      #[doc = concat!("Sets how the container lays out its children (`", stringify!($set), "`).")]
      pub fn $set(&self, p_value: $arg) {
        self.component.$set(p_value);
      }

      #[doc = concat!("Sets `", stringify!($set), "`, and returns the container so calls can be chained.")]
      pub fn $with(self, p_value: $arg) -> Self {
        self.$set(p_value);
        self
      }
    )*
  };
}

/// Holds other components and places them, like a CSS `div` with `display: flex` or `grid`. Stacked is the default: every
/// child at the top left, later ones in front. `with_display` changes that, and the other settings say how:
///
/// - **Flex** puts children in a line along `direction` (a row or a column), spaced by `gap`, moved to the next line when
///   `wrap` is on, and placed along the line with `justify` and across it with `align`.
/// - **Grid** puts children in cells given by `columns` and `rows`; a child can `span` several.
///
/// Like any component it has the box settings (`set_padding`, `set_margin`, `set_border_width`, `set_radius`,
/// `set_background`) and a size, and it goes in a parent with `add`, so containers nest.
///
/// ```ignore
/// let toolbar = Container::new().with_display(Display::Flex).with_gap(8).with_padding(12);
/// toolbar.add(Button::new("Save")).add(slider.fill());
/// page.add(toolbar);
/// ```
#[derive(Clone, Component)]
#[component(plain)]
pub struct Container {
  component: Component,
}

impl Container {
  /// A container that stacks its children. Call `with_display` to place them another way.
  pub fn new() -> Self {
    Self {
      component: Component::new("container"),
    }
  }

  layout! {
    set_display, with_display: Display;
    set_direction, with_direction: Direction;
    set_wrap, with_wrap: bool;
    set_gap, with_gap: u32;
    set_justify, with_justify: Justify;
    set_align, with_align: Align;
  }

  /// Sets the widths of a grid container's columns, for example `[Track::Px(200), Track::Fr(1.0)]`.
  pub fn set_columns(&self, p_columns: impl IntoIterator<Item = Track>) {
    self.component.set_columns(p_columns);
  }

  /// Sets the widths of the columns, and returns the container so calls can be chained.
  pub fn with_columns(self, p_columns: impl IntoIterator<Item = Track>) -> Self {
    self.set_columns(p_columns);
    self
  }

  /// Sets the heights of a grid container's rows.
  pub fn set_rows(&self, p_rows: impl IntoIterator<Item = Track>) {
    self.component.set_rows(p_rows);
  }

  /// Sets the heights of the rows, and returns the container so calls can be chained.
  pub fn with_rows(self, p_rows: impl IntoIterator<Item = Track>) -> Self {
    self.set_rows(p_rows);
    self
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
    let grid = Container::new().with_display(Display::Grid).with_columns([Track::Px(1), Track::Fr(1.0)]);
    grid.size().next((4, 1));
    grid.add(swatch(RED).fill()).add(swatch(BLUE).fill());
    let frame = render(&grid);
    assert_eq!((pixel(&frame, 0, 0), pixel(&frame, 1, 0), pixel(&frame, 3, 0)), (RED, BLUE, BLUE));
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
}
