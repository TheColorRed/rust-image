/// How much of a brush shows at `p_distance` pixels from the middle of a dab or the line of a stroke whose radius is `p_radius`, for a
/// brush `p_hardness` (0.0 soft to 1.0 hard). A soft brush fades like a quadratic, a middle one linearly, and a hard one is solid all the
/// way out to its radius and then cuts off, over about a pixel inside the radius so the cut is not jagged. A brush of size 12 is 12 wide. It is the one falloff every brush shader uses, and the
/// GPU stamping engine (`sprites/sprites.wgsl`) repeats it in WGSL.
pub(crate) fn brush_falloff(p_distance: f32, p_radius: f32, p_hardness: f32) -> f32 {
  if p_radius <= 0.0 || p_distance >= p_radius {
    return 0.0;
  }
  let normalized = p_distance / p_radius;
  let linear = 1.0 - normalized;
  if p_hardness < 0.5 {
    let t = p_hardness * 2.0;
    let quadratic = 1.0 - normalized * normalized;
    (quadratic * (1.0 - t) + linear * t).max(0.0)
  } else {
    let t = (p_hardness - 0.5) * 2.0;
    let ramp = p_radius.clamp(1e-3, 1.0);
    let hard_edge = ((p_radius - p_distance) / ramp).clamp(0.0, 1.0);
    (linear * (1.0 - t) + hard_edge * t).max(0.0)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_middle_is_solid_and_the_edge_is_clear() {
    for hardness in [0.0, 0.3, 0.5, 0.8, 1.0] {
      assert_eq!(brush_falloff(0.0, 10.0, hardness), 1.0);
      assert_eq!(brush_falloff(10.0, 10.0, hardness), 0.0);
      assert_eq!(brush_falloff(15.0, 10.0, hardness), 0.0);
    }
  }

  #[test]
  fn a_hard_brush_is_solid_to_its_radius_and_cuts_off_over_the_last_pixel() {
    assert_eq!(brush_falloff(5.0, 10.0, 1.0), 1.0);
    assert_eq!(brush_falloff(8.9, 10.0, 1.0), 1.0);
    let half_way = brush_falloff(9.5, 10.0, 1.0);
    assert!((half_way - 0.5).abs() < 1e-4, "{half_way}");
    assert_eq!(brush_falloff(10.0, 10.0, 1.0), 0.0);
  }

  #[test]
  fn a_soft_brush_is_fainter_than_a_middle_one_halfway_out() {
    assert!(brush_falloff(5.0, 10.0, 0.0) > brush_falloff(5.0, 10.0, 0.5));
    assert_eq!(brush_falloff(5.0, 10.0, 0.5), 0.5);
  }

  #[test]
  fn a_brush_with_no_radius_shows_nothing() {
    assert_eq!(brush_falloff(0.0, 0.0, 0.5), 0.0);
  }
}
