// Replaces the image with a halftone screen: dots of one ink on a paper color, in rows turned by an angle, each dot as big as the
// image is dark in the cell it sits in. It does the same sums as `apply_halftone` in halftone.rs: the cell's darkness is the mean of
// a grid of `taps` by `taps` pixels spread over the cell, and the share of the cell that is ink is that darkness, as a dot and then as
// gaps closing at the corners. The alpha of the pixel is kept.
// 0 = input, 1 = output, 2 = the settings.
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

struct Settings {
  spacing: f32,
  cos_angle: f32,
  sin_angle: f32,
  taps: u32,
  ink: vec4<f32>,
  paper: vec4<f32>,
}
@group(0) @binding(2) var<uniform> settings: Settings;

fn darkness_at(p: vec2<f32>, dims: vec2<u32>) -> f32 {
  let at = vec2<i32>(clamp(floor(p), vec2<f32>(0.0), vec2<f32>(dims) - vec2<f32>(1.0)));
  let pixel = textureLoad(input_tex, at, 0);
  let luma = 0.299 * pixel.r + 0.587 * pixel.g + 0.114 * pixel.b;
  return (1.0 - luma) * pixel.a;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(gid.xy);
  let source = textureLoad(input_tex, pos, 0);
  let c = settings.cos_angle;
  let s = settings.sin_angle;
  let spacing = settings.spacing;
  let p = vec2<f32>(gid.xy) + vec2<f32>(0.5);
  // The place in the rows of dots: u along a row and v across the rows.
  let u = p.x * c + p.y * s;
  let v = -p.x * s + p.y * c;
  let cu = round(u / spacing) * spacing;
  let cv = round(v / spacing) * spacing;
  let center = vec2<f32>(cu * c - cv * s, cu * s + cv * c);
  var sum = 0.0;
  let taps = f32(settings.taps);
  for (var a = 0u; a < settings.taps; a = a + 1u) {
    for (var b = 0u; b < settings.taps; b = b + 1u) {
      let du = ((f32(a) + 0.5) / taps - 0.5) * spacing;
      let dv = ((f32(b) + 0.5) / taps - 0.5) * spacing;
      sum = sum + darkness_at(center + vec2<f32>(du * c - dv * s, du * s + dv * c), dims);
    }
  }
  let darkness = sum / (taps * taps);
  // The share of the cell that is ink is the share that is dark: a dot of that area up to where the dots touch, and past it the gaps of
  // paper at the corners of the cell.
  let across = u - cu;
  let along = v - cv;
  var coverage = 0.0;
  if (darkness <= 0.78539816) {
    let radius = spacing * sqrt(darkness / 3.14159265) - 0.5 * (1.0 - darkness);
    coverage = clamp(radius - length(vec2<f32>(across, along)) + 0.5, 0.0, 1.0);
  } else {
    let gap = spacing * sqrt((1.0 - darkness) / 3.14159265) - 0.5 * darkness;
    let corner = length(vec2<f32>(spacing * 0.5 - abs(across), spacing * 0.5 - abs(along)));
    coverage = 1.0 - clamp(gap - corner + 0.5, 0.0, 1.0);
  }
  let rgb = mix(settings.paper.rgb, settings.ink.rgb, coverage);
  textureStore(output_tex, pos, vec4<f32>(rgb, source.a));
}
