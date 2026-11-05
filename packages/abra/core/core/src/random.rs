use crate::Image;
use primitives::color::{LumaStandard, luma};

/// A seeded random number generator whose numbers are fixed by this code and not by a dependency, so the same seed gives the same
/// numbers on every machine, in every version, and in any language that implements the same steps (SplitMix64). It is for scattering
/// things repeatably, such as particles, and is not for secrets.
#[derive(Clone, Debug)]
pub struct Random {
  state: u64,
}

impl Random {
  pub fn new(p_seed: u64) -> Self {
    Random { state: p_seed }
  }

  /// The next 64 random bits.
  pub fn next_u64(&mut self) -> u64 {
    self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = self.state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
  }

  /// A number from 0.0 up to, and not including, 1.0.
  pub fn float(&mut self) -> f32 {
    (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
  }

  /// `p_count` numbers from 0.0 up to 1.0, which is one call for a caller that works across a boundary.
  pub fn floats(&mut self, p_count: usize) -> Vec<f32> {
    (0..p_count).map(|_| self.float()).collect()
  }

  /// A number from `p_min` up to `p_max`.
  pub fn range(&mut self, p_min: f32, p_max: f32) -> f32 {
    p_min + (p_max - p_min) * self.float()
  }

  /// A whole number from 0 up to, and not including, `p_len`, which must be more than 0.
  pub fn index(&mut self, p_len: usize) -> usize {
    (((self.next_u64() >> 11) as f64 / (1u64 << 53) as f64 * p_len as f64) as usize).min(p_len.saturating_sub(1))
  }

  /// A point evenly spread over a circle.
  pub fn point_in_circle(&mut self, p_center: (f32, f32), p_radius: f32) -> (f32, f32) {
    let angle = self.range(0.0, std::f32::consts::TAU);
    let distance = p_radius * self.float().sqrt();
    (p_center.0 + angle.cos() * distance, p_center.1 + angle.sin() * distance)
  }

  /// A point evenly spread over the ring between two radii, given in either order.
  pub fn point_in_ring(&mut self, p_center: (f32, f32), p_inner_radius: f32, p_outer_radius: f32) -> (f32, f32) {
    let angle = self.range(0.0, std::f32::consts::TAU);
    let (inner, outer) = (p_inner_radius.min(p_outer_radius), p_outer_radius.max(p_inner_radius));
    let distance = (inner * inner + (outer * outer - inner * inner) * self.float()).sqrt();
    (p_center.0 + angle.cos() * distance, p_center.1 + angle.sin() * distance)
  }

  /// A point evenly spread over a rectangle.
  pub fn point_in_rect(&mut self, p_left: f32, p_top: f32, p_width: f32, p_height: f32) -> (f32, f32) {
    (self.range(p_left, p_left + p_width), self.range(p_top, p_top + p_height))
  }

  /// A point evenly spread along a line.
  pub fn point_on_line(&mut self, p_from: (f32, f32), p_to: (f32, f32)) -> (f32, f32) {
    let t = self.float();
    (p_from.0 + (p_to.0 - p_from.0) * t, p_from.1 + (p_to.1 - p_from.1) * t)
  }

  /// `p_count` points on the parts of an image that are at least this light (0.0 to 1.0), so things come from its bright places, and
  /// the pixels never leave the library. If no pixel is that light the points are anywhere on the image.
  pub fn points_in_light(&mut self, p_image: &Image, p_min_luma: f32, p_count: usize) -> Vec<(f32, f32)> {
    let (width, height) = p_image.dimensions::<usize>();
    let light: Vec<usize> = p_image
      .rgba()
      .chunks_exact(4)
      .enumerate()
      .filter(|(_, pixel)| luma(pixel[0] as f32, pixel[1] as f32, pixel[2] as f32, LumaStandard::Rec709) / 255.0 >= p_min_luma)
      .map(|(at, _)| at)
      .collect();
    (0..p_count)
      .map(|_| match light.is_empty() {
        true => (self.range(0.0, width as f32), self.range(0.0, height as f32)),
        false => {
          let at = light[self.index(light.len())];
          ((at % width) as f32 + self.float(), (at / width) as f32 + self.float())
        }
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Channels;

  #[test]
  fn the_same_seed_gives_the_same_numbers_and_they_are_fixed_forever() {
    let mut a = Random::new(42);
    let mut b = Random::new(42);
    assert_eq!(a.floats(8), b.floats(8));
    // SplitMix64 from seed 0 begins 0xE220A8397B1DCDAF, which any implementation of it must match.
    assert_eq!(Random::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
  }

  #[test]
  fn numbers_stay_in_their_ranges() {
    let mut random = Random::new(1);
    for _ in 0..1000 {
      assert!((0.0..1.0).contains(&random.float()));
      assert!((3.0..5.0).contains(&random.range(3.0, 5.0)));
      assert!(random.index(7) < 7);
    }
  }

  #[test]
  fn points_land_in_their_shapes() {
    let mut random = Random::new(2);
    for _ in 0..500 {
      let (x, y) = random.point_in_circle((10.0, 20.0), 5.0);
      assert!(((x - 10.0).powi(2) + (y - 20.0).powi(2)).sqrt() <= 5.001);
      let (x, y) = random.point_in_ring((0.0, 0.0), 8.0, 4.0);
      let distance = (x * x + y * y).sqrt();
      assert!((3.999..=8.001).contains(&distance));
      let (x, y) = random.point_in_rect(1.0, 2.0, 3.0, 4.0);
      assert!((1.0..=4.0).contains(&x) && (2.0..=6.0).contains(&y));
      let (x, y) = random.point_on_line((0.0, 0.0), (10.0, 10.0));
      assert!((x - y).abs() < 1e-4);
    }
  }

  #[test]
  fn light_points_come_from_the_light_pixels() {
    // Only the right half of the picture is white.
    let pixels: Vec<u8> = (0..16 * 4).flat_map(|i| if i % 16 >= 8 { [255, 255, 255, 255] } else { [0, 0, 0, 255] }).collect();
    let image = Image::new_from_pixels(16, 4, pixels, Channels::RGBA);
    let points = Random::new(3).points_in_light(&image, 0.5, 200);
    assert_eq!(points.len(), 200);
    assert!(points.iter().all(|(x, _)| *x >= 8.0));
    // With nothing that light, they are anywhere.
    let anywhere = Random::new(3).points_in_light(&image, 2.0, 200);
    assert!(anywhere.iter().any(|(x, _)| *x < 8.0));
  }
}
