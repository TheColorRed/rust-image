use abra_core::Image;

// TODO: Fix hue adjustment
/// Adjust the hue of an image where 0.0 is no change, -180.0 is -180 degrees, and 180.0 is 180 degrees.
pub fn hue(_image: &mut Image, _amount: i32) {
  // let amount = (amount as f32).clamp(-180.0, 180.0);
  // let amount = amount / 360.0; // Scale value to range [-0.5, 0.5] where 0.0 means no change

  // let mut colors = image.colors.view_mut();
  // colors.axis_iter_mut(Axis(1)).enumerate().for_each(|(i, mut color)| {
  //   println!("Original Color: {:?}, {:?}, {:?}", color[[0, 0]], color[[0, 1]], color[[0, 2]]);
  //   let (h, s, l) = rgb_to_hsl(color[[0, 0]], color[[0, 1]], color[[0, 2]]);
  //   let (r, g, b) = hsl_to_rgb(h + amount, s, l);
  //   color[[0, 0]] = r;
  //   color[[0, 1]] = g;
  //   color[[0, 2]] = b;
  //   println!("Updated Color: {:?}, {:?}, {:?}", color[[0, 0]], color[[0, 1]], color[[0, 2]]);
  // });

  // for i in 0..(image.color_len) as usize {
  //   let (h, s, l) = rgb_to_hsl(image.r[i], image.g[i], image.b[i]);
  //   let (r, g, b) = hsl_to_rgb(h + amount, s, l);
  //   image.r[i] = r;
  //   image.g[i] = g;
  //   image.b[i] = b;
  // }
}
