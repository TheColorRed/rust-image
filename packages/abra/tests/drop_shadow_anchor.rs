use abra::{
  Canvas,
  canvas::Shadow,
  canvas::{Anchor, DropShadowOptions, Origin},
  color::Color,
  image::Image,
};

#[test]
fn drop_shadow_keeps_anchor_position() {
  let original_size = 100;
  let canvas = Canvas::new_blank("Drop Shadow Anchor", 300, 300).add_layer_from_image(
    "Foreground",
    Image::new_from_color(original_size, original_size, Color::white()),
    None,
  );

  let layer = canvas.get_layer_by_name("Foreground").expect("layer should exist");
  layer.anchor_to_canvas(Anchor::Center);
  layer.set_origin(Origin::Center);

  canvas.update_canvas();
  let position_before = layer.position();

  let distance = 20.0;
  let size = 12.0;
  let angle = 45.0;

  layer
    .effects()
    .drop_shadow(DropShadowOptions::new().with_distance(distance).with_size(size).with_angle(angle));

  canvas.update_canvas();
  let position_after = layer.position();
  let (width_after, height_after) = layer.dimensions::<i32>();

  let angle_rad = angle.to_radians();
  let offset_x = (distance * angle_rad.cos()).round() as i32;
  let offset_y = (distance * angle_rad.sin()).round() as i32;
  let blur_padding = size as i32;

  let expected_padding_left = (-offset_x).max(0) + blur_padding;
  let expected_padding_top = (-offset_y).max(0) + blur_padding;
  let expected_padding_right = offset_x.max(0) + blur_padding;
  let expected_padding_bottom = offset_y.max(0) + blur_padding;

  assert_eq!(position_after.0, position_before.0 - expected_padding_left);
  assert_eq!(position_after.1, position_before.1 - expected_padding_top);
  assert_eq!(width_after, original_size as i32 + expected_padding_left + expected_padding_right);
  assert_eq!(height_after, original_size as i32 + expected_padding_top + expected_padding_bottom);
}
