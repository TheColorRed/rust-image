use crate::common::*;

/// Applies a directional motion blur to an image.
/// - `image`: target image buffer
/// - `p_angle_degrees`: direction of motion in degrees (0 = +X/right)
/// - `p_distance`: length of the blur in pixels (>= 1)
fn apply_motion_blur(img: &mut Image, p_angle_degrees: f32, p_distance: u32) {
  if p_distance == 0 {
    return;
  }
  let (width, height) = img.dimensions::<u32>();
  if width == 0 || height == 0 {
    return;
  }

  let angle_rad = p_angle_degrees.to_radians();
  let dir_x = angle_rad.cos();
  let dir_y = angle_rad.sin();

  // Number of samples along the motion path. We take one sample per pixel distance.
  let samples = p_distance.max(1) as usize;
  let half = (samples as f32 - 1.0) * 0.5; // center the kernel so blur is symmetric

  // Snapshot source pixels once (borrow slice to avoid copying full buffer)
  let src = img.rgba();
  let (w, h) = (width as usize, height as usize);
  let mut out = vec![0u8; w * h * 4];

  out.par_chunks_mut(4).enumerate().for_each(|(idx, dst_px)| {
    let x = (idx % w) as u32;
    let y = (idx / w) as u32;

    let mut acc_r = 0.0f32;
    let mut acc_g = 0.0f32;
    let mut acc_b = 0.0f32;
    let mut acc_a = 0.0f32;

    for i in 0..samples {
      // Offset centered around the current pixel
      let t = i as f32 - half; // range roughly [-half, +half]
      let fx = x as f32 + dir_x * t;
      let fy = y as f32 + dir_y * t;

      // Bilinear sample
      let (r, g, b, a) = {
        let (wi, hi) = (width as i32, height as i32);
        let sx = fx.clamp(0.0, (wi - 1) as f32);
        let sy = fy.clamp(0.0, (hi - 1) as f32);
        let x0 = sx.floor() as i32;
        let y0 = sy.floor() as i32;
        let x1 = (x0 + 1).min(wi - 1);
        let y1 = (y0 + 1).min(hi - 1);
        let tx = sx - x0 as f32;
        let ty = sy - y0 as f32;

        let i00 = ((y0 as usize) * w + x0 as usize) * 4;
        let i10 = ((y0 as usize) * w + x1 as usize) * 4;
        let i01 = ((y1 as usize) * w + x0 as usize) * 4;
        let i11 = ((y1 as usize) * w + x1 as usize) * 4;

        #[inline]
        fn lerp(a: f32, b: f32, t: f32) -> f32 {
          a + (b - a) * t
        }

        let r0 = lerp(src[i00] as f32, src[i10] as f32, tx);
        let g0 = lerp(src[i00 + 1] as f32, src[i10 + 1] as f32, tx);
        let b0 = lerp(src[i00 + 2] as f32, src[i10 + 2] as f32, tx);
        let a0 = lerp(src[i00 + 3] as f32, src[i10 + 3] as f32, tx);

        let r1 = lerp(src[i01] as f32, src[i11] as f32, tx);
        let g1 = lerp(src[i01 + 1] as f32, src[i11 + 1] as f32, tx);
        let b1 = lerp(src[i01 + 2] as f32, src[i11 + 2] as f32, tx);
        let a1 = lerp(src[i01 + 3] as f32, src[i11 + 3] as f32, tx);

        (lerp(r0, r1, ty), lerp(g0, g1, ty), lerp(b0, b1, ty), lerp(a0, a1, ty))
      };

      acc_r += r;
      acc_g += g;
      acc_b += b;
      acc_a += a;
    }

    let inv = 1.0 / samples as f32;
    dst_px[0] = (acc_r * inv).clamp(0.0, 255.0) as u8;
    dst_px[1] = (acc_g * inv).clamp(0.0, 255.0) as u8;
    dst_px[2] = (acc_b * inv).clamp(0.0, 255.0) as u8;
    dst_px[3] = (acc_a * inv).clamp(0.0, 255.0) as u8;
  });

  img.set_rgba_owned(out);
}

/// Applies a motion blur to an image.
/// - `p_image`: The image to be blurred.
/// - `p_angle_degrees`: The angle of the motion blur in degrees.
/// - `p_distance`: The distance of the motion blur in pixels.
/// - `p_apply_options`: Additional options for applying the blur.
pub fn motion_blur<'a>(
  p_image: impl Into<ImageRef<'a>>, p_angle_degrees: f32, p_distance: u32, p_apply_options: impl Into<Options>,
) {
  let mut image_ref: ImageRef = p_image.into();
  let image = &mut image_ref as &mut Image;
  apply_filter!(apply_motion_blur, image, p_apply_options, 1, p_angle_degrees, p_distance);
}
