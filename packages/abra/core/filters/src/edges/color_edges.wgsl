// Draws a line of ink on paper wherever the color of the image changes: a pixel is an edge when its color differs by more than the
// tolerance from the pixel `thickness` pixels to the left of it, or from the pixel `thickness` pixels above it. It does the same sums
// as `apply_color_edges` in color_edges.rs, in whole levels of 0 to 255, so the GPU gives the pixels the CPU does. The alpha of the
// pixel is kept.
// 0 = input, 1 = output, 2 = the settings.
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

struct Settings {
  thickness: u32,
  tolerance: u32,
  pad_a: u32,
  pad_b: u32,
  ink: vec4<f32>,
  paper: vec4<f32>,
}
@group(0) @binding(2) var<uniform> settings: Settings;

fn levels(pos: vec2<i32>) -> vec3<u32> {
  return vec3<u32>(round(textureLoad(input_tex, pos, 0).rgb * 255.0));
}

fn difference(a: vec3<u32>, b: vec3<u32>) -> u32 {
  let d = vec3<i32>(a) - vec3<i32>(b);
  return u32(abs(d.x) + abs(d.y) + abs(d.z));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(gid.xy);
  let source = textureLoad(input_tex, pos, 0);
  let here = levels(pos);
  let t = i32(settings.thickness);
  let left = levels(vec2<i32>(max(pos.x - t, 0), pos.y));
  let above = levels(vec2<i32>(pos.x, max(pos.y - t, 0)));
  // The sum of the three differences is three times their mean, so the tolerance is three times as much.
  let edge = max(difference(here, left), difference(here, above)) > settings.tolerance * 3u;
  let rgb = select(settings.paper.rgb, settings.ink.rgb, edge);
  textureStore(output_tex, pos, vec4<f32>(rgb, source.a));
}
