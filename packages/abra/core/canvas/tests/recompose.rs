use abra_core::{Color, Image};
use canvas::{AddCanvasOptions, Canvas};
use std::sync::Arc;

fn gray(p_value: u8) -> Arc<Image> {
  Arc::new(Image::new_from_color(4, 4, Color::from_rgba(p_value, p_value, p_value, 255)))
}

/// A canvas that has been composed once, with a child canvas in it.
fn composed() -> (Canvas<'static>, Canvas<'static>) {
  let root = Canvas::new_blank("root", 4, 4);
  root.add_layer_from_image("gray", gray(100), None);
  let child = root.add_canvas(Canvas::new_blank("child", 2, 2), AddCanvasOptions::new().with_position(0, 0));
  child.add_layer_from_image("inner", gray(50), None);
  root.as_image();
  assert!(!root.is_dirty(), "a canvas that was just composed is not dirty");
  (root, child)
}

#[test]
fn every_change_to_a_canvas_or_a_canvas_inside_it_makes_it_dirty_until_it_is_composed_again() {
  let (root, child) = composed();
  let changes: [(&str, Box<dyn Fn()>); 5] = [
    ("a layer added to the canvas", Box::new({ let root = root.clone(); move || { root.add_layer_from_image("more", gray(180), None); } })),
    ("a layer added to a canvas inside it", Box::new({ let child = child.clone(); move || { child.add_layer_from_image("more", gray(60), None); } })),
    ("the position of a canvas inside it", Box::new({ let child = child.clone(); move || child.set_position(1, 1) })),
    ("the rotation of a canvas inside it", Box::new({ let child = child.clone(); move || child.set_rotation(Some(10.0)) })),
    ("a canvas added to it", Box::new({ let root = root.clone(); move || { root.add_canvas(Canvas::new_blank("another", 2, 2), None); } })),
  ];
  for (what, change) in changes {
    change();
    assert!(root.is_dirty(), "{what} makes the canvas dirty");
    root.as_image();
    assert!(!root.is_dirty(), "composing clears it after {what}");
  }
}

#[test]
fn a_layer_added_after_the_canvas_was_composed_is_in_the_next_composition() {
  let (root, _child) = composed();
  assert_eq!(root.as_image().get_pixel(3, 3), Some((100, 100, 100, 255)));
  root.add_layer_from_image("lighter", gray(180), None);
  assert_eq!(root.as_image().get_pixel(3, 3), Some((180, 180, 180, 255)));
}

#[test]
fn a_canvas_scaled_inside_another_is_drawn_larger_about_its_middle_and_scaling_back_restores_it() {
  let root = Canvas::new_blank("root", 8, 8);
  // A canvas draws the canvases inside it first and its own layers over them, so the root has none of its own here.
  let child = root.add_canvas(Canvas::new_blank("child", 2, 2), AddCanvasOptions::new().with_position(3, 3));
  child.add_layer_from_image("white", Arc::new(Image::new_from_color(2, 2, Color::from_rgba(255, 255, 255, 255))), None);
  let white = |image: &Image| (0..8).flat_map(|y| (0..8).map(move |x| (x, y))).filter(|&(x, y)| image.get_pixel(x, y).unwrap().0 > 200).count();
  assert_eq!(white(&root.as_image()), 4, "its own size: 2 x 2");

  child.set_scale(2.0);
  assert!(root.is_dirty(), "scaling a canvas inside another makes it dirty");
  let scaled = root.as_image();
  assert_eq!(white(&scaled), 16, "twice the size: 4 x 4");
  assert_eq!(scaled.get_pixel(2, 2).unwrap().0 > 200, true, "it grew from the middle, up and to the left");
  assert_eq!(scaled.get_pixel(5, 5).unwrap().0 > 200, true, "and down and to the right");

  child.set_scale(1.0);
  assert_eq!(white(&root.as_image()), 4, "back to its own size, from its own pixels");
}

#[test]
fn a_canvas_rotated_inside_another_turns_about_its_middle() {
  let root = Canvas::new_blank("root", 40, 40);
  let child = root.add_canvas(Canvas::new_blank("child", 10, 10), AddCanvasOptions::new().with_position(15, 15));
  child.add_layer_from_image("white", Arc::new(Image::new_from_color(10, 10, Color::from_rgba(255, 255, 255, 255))), None);
  let bounds = |image: &Image| {
    let lit: Vec<(u32, u32)> = (0..40).flat_map(|y| (0..40).map(move |x| (x, y))).filter(|&(x, y)| image.get_pixel(x, y).unwrap().0 > 200).collect();
    let (min_x, max_x) = (lit.iter().map(|p| p.0).min().unwrap(), lit.iter().map(|p| p.0).max().unwrap());
    let (min_y, max_y) = (lit.iter().map(|p| p.1).min().unwrap(), lit.iter().map(|p| p.1).max().unwrap());
    (min_x + max_x, min_y + max_y)
  };
  assert_eq!(bounds(&root.as_image()), (39, 39), "the middle of the square, before turning");

  child.set_rotation(Some(45.0));
  let (sum_x, sum_y) = bounds(&root.as_image());
  assert!((sum_x as i32 - 39).abs() <= 1 && (sum_y as i32 - 39).abs() <= 1, "still centered on the same point after turning, got {sum_x},{sum_y}");
}

#[test]
fn a_canvas_both_rotated_and_scaled_keeps_its_middle_where_it_was() {
  for (degrees, scale) in [(30.0, 2.0), (45.0, 0.5), (-70.0, 1.5), (90.0, 2.0)] {
    let root = Canvas::new_blank("root", 60, 60);
    let child = root.add_canvas(Canvas::new_blank("child", 10, 14), AddCanvasOptions::new().with_position(25, 23));
    child.add_layer_from_image("white", Arc::new(Image::new_from_color(10, 14, Color::from_rgba(255, 255, 255, 255))), None);
    child.set_rotation(Some(degrees));
    child.set_scale(scale);
    let image = root.as_image();
    let lit: Vec<(u32, u32)> = (0..60).flat_map(|y| (0..60).map(move |x| (x, y))).filter(|&(x, y)| image.get_pixel(x, y).unwrap().0 > 200).collect();
    let middle_x = (lit.iter().map(|p| p.0).min().unwrap() + lit.iter().map(|p| p.0).max().unwrap()) as f32 / 2.0;
    let middle_y = (lit.iter().map(|p| p.1).min().unwrap() + lit.iter().map(|p| p.1).max().unwrap()) as f32 / 2.0;
    // The canvas's middle is at (25 + 5, 23 + 7) = (30, 30), counting a pixel by its own index.
    assert!((middle_x - 29.5).abs() <= 1.5 && (middle_y - 29.5).abs() <= 1.5, "{degrees} degrees at {scale}x: middle is {middle_x},{middle_y}");
  }
}

