// Paints each pixel with the palette color nearest to it, and keeps alpha. "Nearest" is the distance between the colors in the
// Lab color space, worked out as `rgb_to_lab` does on the CPU (quantize.rs), so the GPU gives the pixels the CPU does.
// 0 = input, 1 = output, 2 = the palette: how many colors it has and the weight of the lightness, then each color's Lab values (with the
// lightness already weighted), then each one's RGB (0 to 1).
// The palette holds at most 64 colors (`MAX_GPU_COLORS` in quantize.rs).
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

struct Palette {
  count: u32,
  lightness: f32,
  lab: array<vec4<f32>, 64>,
  rgb: array<vec4<f32>, 64>,
}
@group(0) @binding(2) var<uniform> palette: Palette;

fn srgb_to_linear(c: f32) -> f32 {
  if (c <= 0.04045) {
    return c / 12.92;
  }
  return pow((c + 0.055) / 1.055, 2.4);
}

fn f_xyz(t: f32) -> f32 {
  let eps = 216.0 / 24389.0;
  let k = 24389.0 / 27.0;
  if (t > eps) {
    return pow(t, 1.0 / 3.0);
  }
  return (k * t + 16.0) / 116.0;
}

fn to_lab(rgb: vec3<f32>) -> vec3<f32> {
  let r = srgb_to_linear(rgb.r);
  let g = srgb_to_linear(rgb.g);
  let b = srgb_to_linear(rgb.b);
  let x = 0.4124564 * r + 0.3575761 * g + 0.1804375 * b;
  let y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
  let z = 0.0193339 * r + 0.1191920 * g + 0.9503041 * b;
  let fx = f_xyz(x / 0.95047);
  let fy = f_xyz(y / 1.00000);
  let fz = f_xyz(z / 1.08883);
  return vec3<f32>((116.0 * fy - 16.0) * palette.lightness, 500.0 * (fx - fy), 200.0 * (fy - fz));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(gid.xy);
  let pixel = textureLoad(input_tex, pos, 0);
  let lab = to_lab(pixel.rgb);
  var best = 0u;
  var best_distance = 3.4e38;
  for (var i = 0u; i < palette.count; i = i + 1u) {
    let d = lab - palette.lab[i].xyz;
    let distance = dot(d, d);
    if (distance < best_distance) {
      best_distance = distance;
      best = i;
    }
  }
  textureStore(output_tex, pos, vec4<f32>(palette.rgb[best].rgb, pixel.a));
}
