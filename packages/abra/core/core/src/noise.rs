/// Smooth noise from -1.0 to 1.0 that changes slowly across `p_x` and `p_y`: a random value at each whole-number point, blended between
/// them. The same `p_seed` gives the same noise.
pub fn value_noise(p_x: f32, p_y: f32, p_seed: u64) -> f32 {
  let corner = |ix: i32, iy: i32| {
    let mut h = (ix as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (iy as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ p_seed;
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    (h >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0
  };
  let (ix, iy) = (p_x.floor(), p_y.floor());
  let ease = |t: f32| t * t * (3.0 - 2.0 * t);
  let (tx, ty) = (ease(p_x - ix), ease(p_y - iy));
  let (ix, iy) = (ix as i32, iy as i32);
  let top = corner(ix, iy) + (corner(ix + 1, iy) - corner(ix, iy)) * tx;
  let bottom = corner(ix, iy + 1) + (corner(ix + 1, iy + 1) - corner(ix, iy + 1)) * tx;
  top + (bottom - top) * ty
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn noise_stays_in_range_and_repeats_for_the_same_seed() {
    for i in 0..200 {
      let (x, y) = (i as f32 * 0.37, i as f32 * 0.91 - 20.0);
      let value = value_noise(x, y, 7);
      assert!((-1.0..=1.0).contains(&value));
      assert_eq!(value, value_noise(x, y, 7));
    }
    assert_ne!(value_noise(1.5, 2.5, 1), value_noise(1.5, 2.5, 2));
  }

  #[test]
  fn noise_changes_slowly() {
    assert!((value_noise(3.0, 4.0, 9) - value_noise(3.01, 4.0, 9)).abs() < 0.1);
  }
}
