struct Params {
  radius: u32, blades: u32, samples: u32, has_specular: u32,
  curvature: f32, rotation: f32, brightness: f32, threshold: f32,
}
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;

fn sample_bilinear(point: vec2<f32>, dims: vec2<i32>) -> vec4<f32> {
  let clamped = clamp(point, vec2<f32>(0.0), vec2<f32>(dims - vec2<i32>(1)));
  let base = vec2<i32>(floor(clamped));
  let next = min(base + vec2<i32>(1), dims - vec2<i32>(1));
  let fraction = clamped - vec2<f32>(base);
  return mix(mix(textureLoad(input_tex, base, 0), textureLoad(input_tex, vec2<i32>(next.x, base.y), 0), fraction.x), mix(textureLoad(input_tex, vec2<i32>(base.x, next.y), 0), textureLoad(input_tex, next, 0), fraction.x), fraction.y);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  let pos = vec2<i32>(gid.xy);
  if (pos.x >= dims.x || pos.y >= dims.y) { return; }
  let count = max(params.samples, 1u);
  let golden = 0.61803398875;
  let pi = 3.14159265359;
  let blades = f32(params.blades);
  var total = vec4<f32>(0.0);
  for (var i = 0u; i < count; i = i + 1u) {
    let unit = sqrt((f32(i) + 0.5) / f32(count));
    let theta = 2.0 * pi * fract(f32(i) * golden) + params.rotation;
    let segment = (theta % (2.0 * pi / blades)) - pi / blades;
    let polygon = cos(pi / blades) * f32(params.radius) / cos(segment);
    let boundary = mix(polygon, f32(params.radius), params.curvature);
    var value = sample_bilinear(vec2<f32>(pos) + vec2<f32>(cos(theta), sin(theta)) * (unit * boundary), dims);
    if (params.has_specular == 1u && dot(value.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) > params.threshold) {
      value = vec4<f32>(value.rgb * params.brightness, value.a);
    }
    total += value;
  }
  textureStore(output_tex, pos, total / f32(count));
}
