/// Fixed-point Gaussian weights with sigma equal to half the radius.
/// The weights sum to 65536, preserving flat areas exactly. Radius zero is identity.
pub fn gaussian_weights(p_radius: u32) -> Vec<u32> {
  if p_radius == 0 {
    return vec![65536];
  }
  let radius = p_radius as i64;
  let sigma = p_radius as f64 / 2.0;
  let raw: Vec<f64> = (-radius..=radius).map(|k| (-((k * k) as f64) / (2.0 * sigma * sigma)).exp()).collect();
  let total: f64 = raw.iter().sum();
  let mut weights: Vec<u32> = raw.iter().map(|value| (value / total * 65536.0).round() as u32).collect();
  let sum: i64 = weights.iter().map(|&weight| weight as i64).sum();
  weights[p_radius as usize] = (weights[p_radius as usize] as i64 + 65536 - sum) as u32;
  weights
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn kernels_preserve_flat_areas_at_supported_tone_radii() {
    for radius in [0, 1, 30, 40, 1024] {
      let weights = gaussian_weights(radius);
      assert_eq!(weights.len(), (2 * radius + 1) as usize);
      assert_eq!(weights.iter().sum::<u32>(), 65536);
      assert!(weights.iter().copied().all(|weight| weight <= 65536));
      assert!(weights.iter().eq(weights.iter().rev()));
    }
  }
}
