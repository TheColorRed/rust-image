use abra_core::Image;
use rayon::prelude::*;

/// Apply a threshold to an image where all pixels above the threshold are set to white and all pixels below are set to black.
/// * `image` - A mutable reference to the image to be thresholded.
/// * `threshold` - The threshold value a value between 0 and 255.
pub fn threshold(image: &mut Image, threshold: u8) {
  let threshold = threshold.clamp(0, 255);
  let pixels = image.colors().as_slice_mut().expect("Image colors must be contiguous");

  pixels.par_chunks_mut(4).for_each(|pixel| {
    let avg = (pixel[0] as f32 + pixel[1] as f32 + pixel[2] as f32) / 3.0;

    if avg > threshold as f32 {
      pixel[0] = 255;
      pixel[1] = 255;
      pixel[2] = 255;
    } else {
      pixel[0] = 0;
      pixel[1] = 0;
      pixel[2] = 0;
    }
  });

  // pixels already mutated in place on the image; no need to set back.
}
