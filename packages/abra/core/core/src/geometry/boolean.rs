//! Boolean operations on polygons: union and difference.
//!
//! A polygon is a list of rings and a point is inside it under the even-odd rule, so a ring inside another one is a hole.
//! The result is returned as rings under the same rule.
//!
//! Every edge is split where it crosses another edge. Each piece is kept when the result changes from one side of it to
//! the other, and the kept pieces are joined back into rings.

use std::collections::{HashMap, HashSet};

use super::PointF;

type Pt = (f64, f64);
type Key = (u64, u64);

/// The union of two polygons: every point inside either one.
/// - `p_a`: The rings of the first polygon.
/// - `p_b`: The rings of the second polygon.
pub fn polygon_union(p_a: &[Vec<PointF>], p_b: &[Vec<PointF>]) -> Vec<Vec<PointF>> {
  combine(p_a, p_b, |in_a, in_b| in_a || in_b)
}

/// The first polygon with the second one taken out of it: every point inside the first and not the second.
/// - `p_a`: The rings of the polygon to cut from.
/// - `p_b`: The rings of the polygon to remove.
pub fn polygon_difference(p_a: &[Vec<PointF>], p_b: &[Vec<PointF>]) -> Vec<Vec<PointF>> {
  combine(p_a, p_b, |in_a, in_b| in_a && !in_b)
}

fn combine(p_a: &[Vec<PointF>], p_b: &[Vec<PointF>], keep: impl Fn(bool, bool) -> bool) -> Vec<Vec<PointF>> {
  let rings_a = to_f64(p_a);
  let rings_b = to_f64(p_b);
  let edges: Vec<(Pt, Pt)> = rings_a.iter().chain(rings_b.iter()).flat_map(|ring| ring_edges(ring)).collect();
  if edges.is_empty() {
    return Vec::new();
  }

  let (min, max) = bounds(&edges);
  let scale = (max.0 - min.0).max(max.1 - min.1).max(1.0);
  let eps = scale * 1e-9;
  let nudge = scale * 1e-7;

  let mut directed = Vec::new();
  for (p, q) in split_edges(&edges, eps).into_iter().flatten() {
    let d = sub(q, p);
    let length = len(d);
    let normal = (-d.1 / length, d.0 / length);
    let mid = ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0);
    let left = (mid.0 + normal.0 * nudge, mid.1 + normal.1 * nudge);
    let right = (mid.0 - normal.0 * nudge, mid.1 - normal.1 * nudge);
    let keep_left = keep(inside(&rings_a, left), inside(&rings_b, left));
    let keep_right = keep(inside(&rings_a, right), inside(&rings_b, right));
    if keep_left != keep_right {
      directed.push(if keep_left { (p, q) } else { (q, p) });
    }
  }

  let mut seen = HashSet::new();
  directed.retain(|&(p, q)| seen.insert((key(p), key(q))));
  link_rings(&directed)
}

fn to_f64(p_rings: &[Vec<PointF>]) -> Vec<Vec<Pt>> {
  p_rings
    .iter()
    .map(|ring| ring.iter().map(|p| (p.x as f64, p.y as f64)).collect())
    .collect()
}

fn ring_edges(p_ring: &[Pt]) -> Vec<(Pt, Pt)> {
  let count = p_ring.len();
  (0..count)
    .map(|i| (p_ring[i], p_ring[(i + 1) % count]))
    .filter(|(p, q)| p != q)
    .collect()
}

fn bounds(p_edges: &[(Pt, Pt)]) -> (Pt, Pt) {
  let mut min = (f64::MAX, f64::MAX);
  let mut max = (f64::MIN, f64::MIN);
  for &(p, q) in p_edges {
    for point in [p, q] {
      min = (min.0.min(point.0), min.1.min(point.1));
      max = (max.0.max(point.0), max.1.max(point.1));
    }
  }
  (min, max)
}

/// Splits every edge at each point where it meets another edge, and returns the pieces of each edge in order.
fn split_edges(p_edges: &[(Pt, Pt)], p_eps: f64) -> Vec<Vec<(Pt, Pt)>> {
  let mut splits: Vec<Vec<(f64, Pt)>> = p_edges.iter().map(|&(a, b)| vec![(0.0, a), (1.0, b)]).collect();

  for i in 0..p_edges.len() {
    for j in i + 1..p_edges.len() {
      let (a, b) = p_edges[i];
      let (c, d) = p_edges[j];
      let r = sub(b, a);
      let s = sub(d, c);
      let len_r = len(r);
      let len_s = len(s);
      let qp = sub(c, a);
      let denom = cross(r, s);

      if denom.abs() > 1e-12 * len_r * len_s {
        let t = cross(qp, s) / denom;
        let u = cross(qp, r) / denom;
        let tol_t = p_eps / len_r;
        let tol_u = p_eps / len_s;
        if t < -tol_t || t > 1.0 + tol_t || u < -tol_u || u > 1.0 + tol_u {
          continue;
        }
        let point = if t <= tol_t {
          a
        } else if t >= 1.0 - tol_t {
          b
        } else if u <= tol_u {
          c
        } else if u >= 1.0 - tol_u {
          d
        } else {
          add(a, scale(r, t))
        };
        splits[i].push((dot(sub(point, a), r) / (len_r * len_r), point));
        splits[j].push((dot(sub(point, c), s) / (len_s * len_s), point));
      } else if cross(qp, r).abs() <= p_eps * len_r {
        for point in [c, d] {
          if let Some(t) = param_on_segment(point, a, r, p_eps) {
            splits[i].push((t, point));
          }
        }
        for point in [a, b] {
          if let Some(u) = param_on_segment(point, c, s, p_eps) {
            splits[j].push((u, point));
          }
        }
      }
    }
  }

  splits
    .into_iter()
    .map(|mut list| {
      list.sort_by(|x, y| x.0.total_cmp(&y.0));
      let mut kept: Vec<Pt> = Vec::new();
      for (_, point) in list {
        if kept.last().is_none_or(|last| dist(*last, point) > p_eps) {
          kept.push(point);
        }
      }
      kept.windows(2).map(|w| (w[0], w[1])).collect()
    })
    .collect()
}

/// The position of `p_point` along the segment that starts at `p_a` with direction `p_r`, when it lies on it.
fn param_on_segment(p_point: Pt, p_a: Pt, p_r: Pt, p_eps: f64) -> Option<f64> {
  let length = len(p_r);
  let t = dot(sub(p_point, p_a), p_r) / (length * length);
  let distance = cross(sub(p_point, p_a), p_r).abs() / length;
  let tol = p_eps / length;
  (distance <= p_eps && t >= -tol && t <= 1.0 + tol).then_some(t)
}

/// Whether a point is inside the rings under the even-odd rule.
fn inside(p_rings: &[Vec<Pt>], p_point: Pt) -> bool {
  let mut inside = false;
  for ring in p_rings {
    let Some(mut previous) = ring.last().copied() else {
      continue;
    };
    for &current in ring {
      if (current.1 > p_point.1) != (previous.1 > p_point.1)
        && p_point.0 < (previous.0 - current.0) * (p_point.1 - current.1) / (previous.1 - current.1) + current.0
      {
        inside = !inside;
      }
      previous = current;
    }
  }
  inside
}

/// Joins directed edges, each with the result on its left, into rings. At a vertex with several ways on, the ring
/// takes the sharpest left turn so that separate parts stay separate.
fn link_rings(p_edges: &[(Pt, Pt)]) -> Vec<Vec<PointF>> {
  let mut outgoing: HashMap<Key, Vec<usize>> = HashMap::new();
  for (index, &(p, _)) in p_edges.iter().enumerate() {
    outgoing.entry(key(p)).or_default().push(index);
  }

  let mut used = vec![false; p_edges.len()];
  let mut rings = Vec::new();
  for start in 0..p_edges.len() {
    if used[start] {
      continue;
    }
    let mut ring = Vec::new();
    let mut current = start;
    loop {
      used[current] = true;
      let (p, q) = p_edges[current];
      ring.push(p);
      let incoming = sub(q, p);
      let next = outgoing.get(&key(q)).and_then(|candidates| {
        candidates
          .iter()
          .copied()
          .filter(|&candidate| candidate == start || !used[candidate])
          .max_by(|&x, &y| {
            let turn_x = turn(incoming, sub(p_edges[x].1, p_edges[x].0));
            let turn_y = turn(incoming, sub(p_edges[y].1, p_edges[y].0));
            turn_x.total_cmp(&turn_y)
          })
      });
      match next {
        Some(next) if next != start => current = next,
        _ => break,
      }
    }
    if ring.len() >= 3 {
      rings.push(ring.into_iter().map(|p| PointF::new(p.0 as f32, p.1 as f32)).collect());
    }
  }
  rings
}

fn turn(p_incoming: Pt, p_outgoing: Pt) -> f64 {
  cross(p_incoming, p_outgoing).atan2(dot(p_incoming, p_outgoing))
}

fn key(p_point: Pt) -> Key {
  ((p_point.0 + 0.0).to_bits(), (p_point.1 + 0.0).to_bits())
}

fn sub(a: Pt, b: Pt) -> Pt {
  (a.0 - b.0, a.1 - b.1)
}

fn add(a: Pt, b: Pt) -> Pt {
  (a.0 + b.0, a.1 + b.1)
}

fn scale(a: Pt, factor: f64) -> Pt {
  (a.0 * factor, a.1 * factor)
}

fn dot(a: Pt, b: Pt) -> f64 {
  a.0 * b.0 + a.1 * b.1
}

fn cross(a: Pt, b: Pt) -> f64 {
  a.0 * b.1 - a.1 * b.0
}

fn len(a: Pt) -> f64 {
  dot(a, a).sqrt()
}

fn dist(a: Pt, b: Pt) -> f64 {
  len(sub(a, b))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn square(x: f32, y: f32, size: f32) -> Vec<PointF> {
    vec![
      PointF::new(x, y),
      PointF::new(x + size, y),
      PointF::new(x + size, y + size),
      PointF::new(x, y + size),
    ]
  }

  /// The area of the rings, with outer rings counted positive and holes negative as the even-odd rule implies.
  fn area(p_rings: &[Vec<PointF>]) -> f64 {
    p_rings
      .iter()
      .map(|ring| {
        let pts = to_f64(std::slice::from_ref(ring)).remove(0);
        let count = pts.len();
        (0..count)
          .map(|i| cross(pts[i], pts[(i + 1) % count]))
          .sum::<f64>()
          / 2.0
      })
      .sum::<f64>()
      .abs()
  }

  #[test]
  fn union_of_overlapping_squares() {
    let result = polygon_union(&[square(0.0, 0.0, 10.0)], &[square(5.0, 5.0, 10.0)]);
    assert_eq!(result.len(), 1);
    assert!((area(&result) - 175.0).abs() < 1e-3);
  }

  #[test]
  fn union_of_separate_squares_keeps_both() {
    let result = polygon_union(&[square(0.0, 0.0, 10.0)], &[square(20.0, 0.0, 10.0)]);
    assert_eq!(result.len(), 2);
    assert!((area(&result) - 200.0).abs() < 1e-3);
  }

  #[test]
  fn difference_with_inner_square_makes_a_hole() {
    let result = polygon_difference(&[square(0.0, 0.0, 10.0)], &[square(3.0, 3.0, 4.0)]);
    assert_eq!(result.len(), 2);
    assert!((area(&result) - 84.0).abs() < 1e-3);
  }

  #[test]
  fn difference_across_the_edge_leaves_a_notch() {
    let result = polygon_difference(&[square(0.0, 0.0, 10.0)], &[square(5.0, 5.0, 10.0)]);
    assert_eq!(result.len(), 1);
    assert!((area(&result) - 75.0).abs() < 1e-3);
  }

  #[test]
  fn difference_with_disjoint_square_keeps_the_first() {
    let result = polygon_difference(&[square(0.0, 0.0, 10.0)], &[square(20.0, 0.0, 10.0)]);
    assert_eq!(result.len(), 1);
    assert!((area(&result) - 100.0).abs() < 1e-3);
  }

  #[test]
  fn difference_with_itself_is_empty() {
    let result = polygon_difference(&[square(0.0, 0.0, 10.0)], &[square(0.0, 0.0, 10.0)]);
    assert!(result.is_empty());
  }

  #[test]
  fn union_with_itself_is_unchanged() {
    let result = polygon_union(&[square(0.0, 0.0, 10.0)], &[square(0.0, 0.0, 10.0)]);
    assert_eq!(result.len(), 1);
    assert!((area(&result) - 100.0).abs() < 1e-3);
  }

  #[test]
  fn union_with_nothing_is_the_other_shape() {
    let result = polygon_union(&[], &[square(0.0, 0.0, 10.0)]);
    assert_eq!(result.len(), 1);
    assert!((area(&result) - 100.0).abs() < 1e-3);
  }

  #[test]
  fn difference_from_nothing_is_empty() {
    assert!(polygon_difference(&[], &[square(0.0, 0.0, 10.0)]).is_empty());
  }

  #[test]
  fn outer_ring_and_hole_wind_opposite_ways() {
    let result = polygon_difference(&[square(0.0, 0.0, 10.0)], &[square(3.0, 3.0, 4.0)]);
    let signs: Vec<f64> = result.iter().map(|ring| signed_area(ring)).collect();
    let outer = signs.iter().copied().max_by(|a, b| a.abs().total_cmp(&b.abs())).unwrap();
    assert!(outer > 0.0);
    assert!(signs.iter().filter(|s| **s != outer).all(|s| *s < 0.0));
  }

  /// The shoelace area of one ring, positive when it winds counter-clockwise in the y-up convention.
  fn signed_area(p_ring: &[PointF]) -> f64 {
    let pts = to_f64(&[p_ring.to_vec()]).remove(0);
    let count = pts.len();
    (0..count).map(|i| cross(pts[i], pts[(i + 1) % count])).sum::<f64>() / 2.0
  }
}
