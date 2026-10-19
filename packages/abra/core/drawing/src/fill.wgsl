// Fills an area with a color, a gradient or an image, matching the CPU rasterizer of `fill.rs` step by step: each pixel of the area is
// sampled on a `side` x `side` grid, every sample inside the shape (non-zero winding) is shaded, the shaded colors are averaged and the
// number of samples inside says how much of the color shows. The input is only there for its size. Pixels away from the shape are
// transparent, so the result is made to be blended over a photo.
//
// Everything is in the area's own pixels ("local"): `offset` is where the area's top-left lands in the photo, a whole number of pixels,
// and `window` says where this image sits in the photo, so the shape is the same however the photo is zoomed.
//
// Binding 3 is the image of an image fill, or the mask (its red channel) of a masked fill, or one clear pixel when there is neither.
struct Stop {
  // 0 to 255 each.
  color: vec4<f32>,
  // The position of the stop in `x`.
  time: vec4<f32>,
}

struct Params {
  // The color of a solid fill, 0 to 255 each.
  color: vec4<f32>,
  // Where the area's top-left lands in the photo.
  offset: vec2<f32>,
  // The size of the area in pixels.
  size: vec2<f32>,
  // The points of the shape.
  count: u32,
  // 0 is a solid color, 1 a gradient and 2 an image.
  kind: u32,
  stop_count: u32,
  // The points of the gradient's line, and how many pieces it has (all of them, or one less when it is not closed).
  direction_count: u32,
  direction_segments: u32,
  // Samples per side of a pixel.
  side: u32,
  // 1 when binding 3 is a mask.
  masked: u32,
  // Bit 0 tiles the image horizontally, bit 1 vertically. Used for an image fill that is not masked.
  repeat_bits: u32,
  // How far in from the edge, in pixels, the fill fades to its full opacity. 0 for none.
  feather: f32,
  pad1: f32,
  pad2: f32,
  pad3: f32,
  stops: array<Stop, 16>,
  direction: array<vec4<f32>, 64>,
  points: array<vec4<f32>, 1024>,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var aux_tex: texture_2d<f32>;
struct Window {
  origin: vec2<f32>,
  scale: f32,
  pad: f32,
  full: vec2<f32>,
  pad2: vec2<f32>,
}
@group(0) @binding(5) var<uniform> window: Window;

// Whether `p` is inside the shape, by the non-zero winding rule.
fn contains(p: vec2<f32>) -> bool {
  var winding = 0;
  var j = params.count - 1u;
  for (var i = 0u; i < params.count; i = i + 1u) {
    let a = params.points[i].xy;
    let b = params.points[j].xy;
    if (a.y <= p.y) {
      if (b.y > p.y) {
        if ((b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y) > 0.0) {
          winding = winding + 1;
        }
      }
    } else {
      if (b.y <= p.y) {
        if ((b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y) < 0.0) {
          winding = winding - 1;
        }
      }
    }
    j = i;
  }
  return winding != 0;
}

// How far `p` is from the edge, along the points as an open line (the last point is not joined to the first).
fn edge_distance(p: vec2<f32>) -> f32 {
  var nearest = 1.0e30;
  for (var i = 0u; i + 1u < params.count; i = i + 1u) {
    let a = params.points[i].xy;
    let ab = params.points[i + 1u].xy - a;
    let length_squared = dot(ab, ab);
    if (length_squared == 0.0) {
      continue;
    }
    let along = clamp(dot(p - a, ab) / length_squared, 0.0, 1.0);
    nearest = min(nearest, distance(p, a + ab * along));
  }
  return nearest;
}

// Where along the gradient's line `p` is, from 0 to 1.
fn gradient_time(p: vec2<f32>) -> f32 {
  var best = 1.0e30;
  var time = 0.0;
  for (var i = 0u; i < params.direction_segments; i = i + 1u) {
    let a = params.direction[i].xy;
    let ab = params.direction[(i + 1u) % params.direction_count].xy - a;
    let length_squared = dot(ab, ab);
    var along = 0.0;
    if (length_squared >= 1.0e-4) {
      along = clamp(dot(p - a, ab) / length_squared, 0.0, 1.0);
    }
    let d = distance(p, a + ab * along);
    if (d < best) {
      best = d;
      time = (f32(i) + along) / f32(params.direction_segments);
    }
  }
  return time;
}

// The color of the gradient at `time`: the stops around it blended, or the first or last stop's color past the ends.
fn gradient_color(time: f32) -> vec4<f32> {
  if (params.stop_count == 0u) {
    return vec4<f32>(0.0);
  }
  var after = params.stop_count;
  for (var i = 0u; i < params.stop_count; i = i + 1u) {
    if (params.stops[i].time.x > time) {
      after = i;
      break;
    }
  }
  if (after == 0u) {
    return params.stops[0].color;
  }
  if (after == params.stop_count) {
    return params.stops[params.stop_count - 1u].color;
  }
  let start = params.stops[after - 1u];
  let end = params.stops[after];
  let t = (time - start.time.x) / (end.time.x - start.time.x);
  return floor(start.color + (end.color - start.color) * t);
}

// The color of the fill at `p` (local), 0 to 255 each.
fn shade(p: vec2<f32>) -> vec4<f32> {
  var color = params.color;
  if (params.kind == 1u) {
    color = gradient_color(gradient_time(p));
  } else if (params.kind == 2u) {
    var at = vec2<i32>(floor(p));
    let dims = vec2<i32>(textureDimensions(aux_tex));
    // Euclidean modulo, so a negative coordinate wraps forwards.
    if ((params.repeat_bits & 1u) != 0u) {
      at.x = ((at.x % dims.x) + dims.x) % dims.x;
    }
    if ((params.repeat_bits & 2u) != 0u) {
      at.y = ((at.y % dims.y) + dims.y) % dims.y;
    }
    if (at.x < 0 || at.y < 0 || at.x >= dims.x || at.y >= dims.y) {
      color = vec4<f32>(0.0);
    } else {
      color = round(textureLoad(aux_tex, at, 0) * 255.0);
    }
  }
  if (params.feather > 0.0) {
    color.a = floor(color.a * clamp(edge_distance(p) / params.feather, 0.0, 1.0));
  }
  return color;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let at = vec2<i32>(i32(gid.x), i32(gid.y));
  let corner = window.origin + vec2<f32>(f32(gid.x), f32(gid.y)) * window.scale - params.offset;
  if (params.count < 3u || corner.x < 0.0 || corner.y < 0.0 || corner.x >= params.size.x || corner.y >= params.size.y) {
    textureStore(output_tex, at, vec4<f32>(0.0));
    return;
  }
  let side = f32(params.side);
  var inside = 0.0;
  var sum = vec4<f32>(0.0);
  for (var sy = 0u; sy < params.side; sy = sy + 1u) {
    for (var sx = 0u; sx < params.side; sx = sx + 1u) {
      let p = corner + (vec2<f32>(f32(sx), f32(sy)) + 0.5) / side * window.scale;
      if (contains(p)) {
        inside = inside + 1.0;
        sum = sum + shade(p);
      }
    }
  }
  if (inside == 0.0) {
    textureStore(output_tex, at, vec4<f32>(0.0));
    return;
  }
  let average = floor(sum / inside);
  var alpha = round(average.a / 255.0 * (inside / (side * side)) * 255.0);
  if (params.masked == 1u) {
    let mask_at = vec2<i32>(floor(corner + 0.5 * window.scale));
    let mask = round(textureLoad(aux_tex, mask_at, 0).r * 255.0);
    alpha = floor((alpha * mask + 127.0) / 255.0);
  }
  if (alpha <= 0.0) {
    textureStore(output_tex, at, vec4<f32>(0.0));
  } else {
    textureStore(output_tex, at, vec4<f32>(average.rgb / 255.0, alpha / 255.0));
  }
}
