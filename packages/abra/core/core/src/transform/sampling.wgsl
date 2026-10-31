// The samplers resize.wgsl and rotate.wgsl share, with the same steps as the samplers in interpolation.rs on the CPU: premultiplied
// colors, divided by the weights' sum where there are weights, then un-premultiplied and rounded. The window does not widen when the
// image shrinks, as on the CPU. Pixels past the edges are transparent.
//
// The file is put in front of a shader (`concat!` with `include_str!`), which has its own uniforms and `main`.

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

const PI: f32 = 3.14159265358979;

fn tent(t: f32) -> f32 {
  return max(1.0 - abs(t), 0.0);
}

fn cubic(t: f32) -> f32 {
  let a = abs(t);
  if (a < 1.0) {
    return 1.0 - 2.0 * a * a + a * a * a;
  }
  if (a < 2.0) {
    return -4.0 + 8.0 * a - 5.0 * a * a + a * a * a;
  }
  return 0.0;
}

fn lanczos3(t: f32) -> f32 {
  let a = abs(t);
  if (a == 0.0) {
    return 1.0;
  }
  if (a < 3.0) {
    let pi_a = PI * a;
    return (sin(pi_a) / pi_a) * (sin(pi_a / 3.0) / (pi_a / 3.0));
  }
  return 0.0;
}

// The weight of the kernel (1 bilinear's tent, 2 bicubic's cubic, 3 lanczos) at a distance of `t` pixels.
fn weight(kernel: u32, t: f32) -> f32 {
  if (kernel == 1u) {
    return tent(t);
  }
  if (kernel == 2u) {
    return cubic(t);
  }
  return lanczos3(t);
}

// The pixel at `at` as its four values, 0 to 255, or transparent past the edges.
fn texel(at: vec2<i32>, dims: vec2<i32>) -> vec4<f32> {
  if (at.x < 0 || at.y < 0 || at.x >= dims.x || at.y >= dims.y) {
    return vec4<f32>(0.0);
  }
  return round(textureLoad(input_tex, at, 0) * 255.0);
}

// The pixel with its color multiplied by its alpha (0 to 1), as `[r * a, g * a, b * a, a]`.
fn premultiplied(at: vec2<i32>, dims: vec2<i32>) -> vec4<f32> {
  let pixel = texel(at, dims);
  let alpha = pixel.a / 255.0;
  return vec4<f32>(pixel.rgb * alpha, alpha);
}

// The brightness of the pixel at `at` without its alpha, Rec. 601 weights, 0 to 255.
fn luma_at(at: vec2<i32>, dims: vec2<i32>) -> f32 {
  let pixel = premultiplied(at, dims);
  if (pixel.a > 0.0) {
    return (0.299 * pixel.r + 0.587 * pixel.g + 0.114 * pixel.b) / pixel.a;
  }
  return 0.0;
}

// Blends the 2 by 2 premultiplied pixels around a spot between pixels.
fn premultiplied_bilinear(spot: vec2<f32>, dims: vec2<i32>) -> vec4<f32> {
  let base = vec2<i32>(floor(spot));
  let fraction = spot - vec2<f32>(base);
  let top_left = premultiplied(base, dims);
  let top_right = premultiplied(base + vec2<i32>(1, 0), dims);
  let bottom_left = premultiplied(base + vec2<i32>(0, 1), dims);
  let bottom_right = premultiplied(base + vec2<i32>(1, 1), dims);
  let top = top_left + (top_right - top_left) * fraction.x;
  let bottom = bottom_left + (bottom_right - bottom_left) * fraction.x;
  return top + (bottom - top) * fraction.y;
}

// A premultiplied color back to a straight one, 0 to 255 and rounded, or transparent when there is no alpha.
fn unpremultiply(sum: vec4<f32>) -> vec4<f32> {
  if (sum.a <= 0.0) {
    return vec4<f32>(0.0);
  }
  let color = clamp(sum.rgb / sum.a, vec3<f32>(0.0), vec3<f32>(255.0));
  return vec4<f32>(floor(color + 0.5), floor(clamp(sum.a * 255.0, 0.0, 255.0) + 0.5));
}

// The weighted window around the spot, for the kernel (1 bilinear, 2 bicubic, 3 lanczos).
fn sample_kernel(base: vec2<i32>, fraction: vec2<f32>, kernel: u32, dims: vec2<i32>) -> vec4<f32> {
  let first = select(select(-2, -1, kernel == 2u), 0, kernel == 1u);
  let last = select(select(3, 2, kernel == 2u), 1, kernel == 1u);
  var sum = vec4<f32>(0.0);
  var total = 0.0;
  for (var dy = first; dy <= last; dy = dy + 1) {
    let wy = weight(kernel, f32(dy) - fraction.y);
    for (var dx = first; dx <= last; dx = dx + 1) {
      let w = weight(kernel, f32(dx) - fraction.x) * wy;
      sum = sum + premultiplied(base + vec2<i32>(dx, dy), dims) * w;
      total = total + w;
    }
  }
  if (total > 0.0) {
    sum = sum / total;
  }
  return unpremultiply(sum);
}

// New Edge-Directed Interpolation: the direction of the edge comes from the covariance of the brightness gradients in a 5 by 5
// window, and the result is a blend of two spots along it. Where the edge is weak it is bicubic.
fn sample_nedi(base: vec2<i32>, fraction: vec2<f32>, dims: vec2<i32>) -> vec4<f32> {
  var gradient_x: array<f32, 25>;
  var gradient_y: array<f32, 25>;
  var index = 0;
  for (var dy = -2; dy <= 2; dy = dy + 1) {
    for (var dx = -2; dx <= 2; dx = dx + 1) {
      let at = base + vec2<i32>(dx, dy);
      gradient_x[index] = (luma_at(at + vec2<i32>(1, 0), dims) - luma_at(at - vec2<i32>(1, 0), dims)) * 0.5;
      gradient_y[index] = (luma_at(at + vec2<i32>(0, 1), dims) - luma_at(at - vec2<i32>(0, 1), dims)) * 0.5;
      index = index + 1;
    }
  }

  var sum_x = 0.0;
  var sum_y = 0.0;
  for (var i = 0; i < 25; i = i + 1) {
    sum_x = sum_x + gradient_x[i];
    sum_y = sum_y + gradient_y[i];
  }
  let mean_x = sum_x / 25.0;
  let mean_y = sum_y / 25.0;
  var a = 0.0;
  var b = 0.0;
  var c = 0.0;
  for (var i = 0; i < 25; i = i + 1) {
    let delta_x = gradient_x[i] - mean_x;
    let delta_y = gradient_y[i] - mean_y;
    a = a + delta_x * delta_x;
    b = b + delta_x * delta_y;
    c = c + delta_y * delta_y;
  }
  a = a / 25.0;
  b = b / 25.0;
  c = c / 25.0;

  if (sqrt(a + c) <= 5.0) {
    return sample_kernel(base, fraction, 2u, dims);
  }

  // The eigenvector of the larger eigenvalue of the covariance is the main edge direction.
  let trace = a + c;
  let discriminant = sqrt(max(trace * trace * 0.25 - (a * c - b * b), 0.0));
  let lambda1 = trace * 0.5 + discriminant;
  let lambda2 = trace * 0.5 - discriminant;
  let lambda = select(lambda2, lambda1, abs(lambda1) > abs(lambda2));
  var edge = vec2<f32>(1.0, 0.0);
  if (abs(b) > 1e-6) {
    let vector = vec2<f32>(lambda - c, b);
    let norm = sqrt(vector.x * vector.x + vector.y * vector.y);
    if (norm > 0.0) {
      edge = vector / norm;
    }
  } else if (abs(a - c) > 1e-6) {
    edge = select(vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), a > c);
  }

  let behind = premultiplied_bilinear(vec2<f32>(base) - edge, dims);
  let ahead = premultiplied_bilinear(vec2<f32>(base) + edge, dims);
  let t = ((fraction.x * edge.x + fraction.y * edge.y) + 1.0) * 0.5;
  return unpremultiply(behind + (ahead - behind) * t);
}

// Edge-Directed Interpolation: the Sobel direction of the edge, snapped to one of four, picks the two pixels to blend. Where the edge
// is weak it is bilinear.
fn sample_edi(base: vec2<i32>, fraction: vec2<f32>, dims: vec2<i32>) -> vec4<f32> {
  let gx = -luma_at(base + vec2<i32>(-1, -1), dims) - 2.0 * luma_at(base + vec2<i32>(-1, 0), dims) - luma_at(base + vec2<i32>(-1, 1), dims)
    + luma_at(base + vec2<i32>(1, -1), dims) + 2.0 * luma_at(base + vec2<i32>(1, 0), dims) + luma_at(base + vec2<i32>(1, 1), dims);
  let gy = -luma_at(base + vec2<i32>(-1, -1), dims) - 2.0 * luma_at(base + vec2<i32>(0, -1), dims) - luma_at(base + vec2<i32>(1, -1), dims)
    + luma_at(base + vec2<i32>(-1, 1), dims) + 2.0 * luma_at(base + vec2<i32>(0, 1), dims) + luma_at(base + vec2<i32>(1, 1), dims);

  if (sqrt(gx * gx + gy * gy) <= 10.0) {
    return sample_kernel(base, fraction, 1u, dims);
  }

  var angle = atan2(gy, gx);
  if (angle < 0.0) {
    angle = angle + PI;
  }
  let direction = i32(floor(angle / PI * 4.0 + 0.5)) % 4;
  var start = vec2<i32>(1, 0);
  var end = vec2<i32>(0, 1);
  var t = (fraction.x + fraction.y) * 0.5;
  if (direction == 0) {
    start = vec2<i32>(0, 0);
    end = vec2<i32>(1, 0);
    t = fraction.x;
  } else if (direction == 1) {
    start = vec2<i32>(0, 0);
    end = vec2<i32>(1, 1);
  } else if (direction == 2) {
    start = vec2<i32>(0, 0);
    end = vec2<i32>(0, 1);
    t = fraction.y;
  }
  let from_start = premultiplied(base + start, dims);
  let to_end = premultiplied(base + end, dims);
  return unpremultiply(from_start + (to_end - from_start) * t);
}
