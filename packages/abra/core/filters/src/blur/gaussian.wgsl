struct Params {
  radius: i32,
  // 0 blurs horizontally, 1 blurs vertically.
  direction: u32,
  pad0: u32,
  pad1: u32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;
// The weights, 2 * radius + 1 texels, three bytes each (red is the low byte). They are whole numbers that add up to
// exactly 65536, built on the CPU by `gaussian_weights` in gaussian.rs.
@group(0) @binding(3) var weights_tex: texture_2d<f32>;
// Where this image sits in the photo it is a view of: pixel `p` is the photo's pixel `origin + (p + 0.5) * scale`. The radius
// is in the photo's pixels, so at another scale the same blur is a different number of this image's pixels.
struct Window {
  origin: vec2<f32>,
  scale: f32,
  pad: f32,
  full: vec2<f32>,
  pad2: vec2<f32>,
}
@group(0) @binding(5) var<uniform> window: Window;

// The same blur at a scale other than 1, in floating point (it is a view and not the saved photo, so it only has to look
// the same). The kernel reaches `radius / scale` of this image's pixels, which is more than there is time for when the view is
// deep in the photo (and this runs every frame of a drag), so it looks at no more than 8 pixels each side and spaces them out
// to cover the reach; a Gaussian that wide is smooth enough that it cannot tell, since the taps are then a quarter of its
// width apart. Each tap is read between two pixels, and weighs the Gaussian's curve for its distance in photo pixels (the
// table's sigma is half the radius).
fn blur_scaled(pos: vec2<i32>, dims: vec2<i32>, step: vec2<i32>) -> vec4<f32> {
  let radius = params.radius;
  let reach = f32(radius) / window.scale;
  let spacing = max(1.0, reach / 8.0);
  let taps = i32(floor(reach / spacing));
  let last = dims - vec2<i32>(1);
  let sigma = max(f32(radius) * 0.5, 0.5);
  var sum = vec4<f32>(0.0);
  var total = 0.0;
  for (var j = -taps; j <= taps; j = j + 1) {
    let offset = f32(j) * spacing;
    let distance = offset * window.scale;
    let weight = exp(-(distance * distance) / (2.0 * sigma * sigma));
    let tap = vec2<f32>(pos) + vec2<f32>(step) * offset;
    let base = vec2<i32>(floor(tap));
    let across = dot(tap - vec2<f32>(base), vec2<f32>(step));
    let a = textureLoad(input_tex, clamp(base, vec2<i32>(0), last), 0);
    let b = textureLoad(input_tex, clamp(base + step, vec2<i32>(0), last), 0);
    sum = sum + mix(a, b, across) * weight;
    total = total + weight;
  }
  return sum / max(total, 1.0);
}

// One pass of a separable Gaussian blur in whole numbers, the same steps as the CPU path: the weights times the pixels
// around it (edge pixels clamped), rounded. Nothing here is floating point except reading and writing 8-bit values,
// so the GPU gives exactly the CPU's pixels.
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  let pos = vec2<i32>(gid.xy);
  if (pos.x >= dims.x || pos.y >= dims.y) {
    return;
  }

  let radius = params.radius;
  var step = vec2<i32>(1, 0);
  if (params.direction == 1u) {
    step = vec2<i32>(0, 1);
  }

  if (abs(window.scale - 1.0) > 0.0001) {
    textureStore(output_tex, pos, blur_scaled(pos, dims, step));
    return;
  }

  var sums = vec4<u32>(0u);
  for (var k = -radius; k <= radius; k = k + 1) {
    let sample_pos = clamp(pos + step * k, vec2<i32>(0), dims - vec2<i32>(1));
    let bytes = vec3<u32>(round(textureLoad(weights_tex, vec2<i32>(k + radius, 0), 0).rgb * 255.0));
    let weight = bytes.r | (bytes.g << 8u) | (bytes.b << 16u);
    let pixel = vec4<u32>(round(textureLoad(input_tex, sample_pos, 0) * 255.0));
    sums = sums + pixel * weight;
  }
  let result = (sums + vec4<u32>(32768u)) >> vec4<u32>(16u);
  textureStore(output_tex, pos, vec4<f32>(result) / 255.0);
}
