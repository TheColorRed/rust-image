use crate::image::Image;

/// Adjusts the contrast of an image.
pub fn contrast(img: &mut Image, mut amount: i32) {
  amount = amount.clamp(-100, 100);
  let factor = (259 * (amount + 255)) / (255 * (259 - amount));
  img
    .colors
    .par_map_inplace(|x| *x = (factor * (*x as i32 - 128) + 128).clamp(0, 255) as u8);
  // let (width, height) = img.dimensions();
  // let factor = (259 * (amount + 255)) / (255 * (259 - amount));
  // for i in 0..(width * height) as usize {
  //   img.r[i] = (factor * (img.r[i] as i32 - 128) + 128).clamp(0, 255) as u8;
  //   img.g[i] = (factor * (img.g[i] as i32 - 128) + 128).clamp(0, 255) as u8;
  //   img.b[i] = (factor * (img.b[i] as i32 - 128) + 128).clamp(0, 255) as u8;
  // }
}
