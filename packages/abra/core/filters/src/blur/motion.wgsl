struct Params { angle: f32, distance: u32, pad0: u32, pad1: u32 }
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
  let samples = max(params.distance, 1u);
  let half = (f32(samples) - 1.0) * 0.5;
  let direction = vec2<f32>(cos(params.angle), sin(params.angle));
  var total = vec4<f32>(0.0);
  for (var i = 0u; i < samples; i = i + 1u) {
    total += sample_bilinear(vec2<f32>(pos) + direction * (f32(i) - half), dims);
  }
  textureStore(output_tex, pos, total / f32(samples));
}
