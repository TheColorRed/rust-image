struct Params { radius: i32, direction: u32, pad0: u32, pad1: u32 }
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;
struct Window { origin: vec2<f32>, scale: f32, pad: f32, full: vec2<f32>, pad2: vec2<f32> }
@group(0) @binding(5) var<uniform> window: Window;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  let pos = vec2<i32>(gid.xy);
  if (pos.x >= dims.x || pos.y >= dims.y) { return; }
  var step = vec2<i32>(1, 0);
  if (params.direction == 1u) { step = vec2<i32>(0, 1); }
  var taps = params.radius;
  var spacing = 1.0;
  if (abs(window.scale - 1.0) > 0.0001) {
    let reach = f32(params.radius) / window.scale;
    spacing = max(1.0, reach / 32.0);
    taps = i32(floor(reach / spacing));
  }
  var sum = vec4<u32>(0u);
  var count = 0u;
  for (var k = -taps; k <= taps; k = k + 1) {
    let offset = i32(round(f32(k) * spacing));
    let at = clamp(pos + step * offset, vec2<i32>(0), dims - vec2<i32>(1));
    sum += vec4<u32>(round(textureLoad(input_tex, at, 0) * 255.0));
    count += 1u;
  }
  textureStore(output_tex, pos, vec4<f32>(sum / vec4<u32>(count)) / 255.0);
}
