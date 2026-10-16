struct Params {
  radius: i32,
  // 0 averages along the rows, 1 down the columns.
  direction: u32,
  pad0: u32,
  pad1: u32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba32float, write>;
@group(0) @binding(2) var<uniform> params: Params;

// One pass of the guided filter's box mean, on all four numbers of a pixel at once: the average of the pixels within the radius
// along the row or the column. Pixels past the edge are left out of the average (not repeated), as the CPU does, so a pixel at the
// edge averages fewer pixels. Two passes, one each way, give the average over a square window.
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  let pos = vec2<i32>(gid.xy);
  if (pos.x >= dims.x || pos.y >= dims.y) {
    return;
  }
  var step = vec2<i32>(1, 0);
  if (params.direction == 1u) {
    step = vec2<i32>(0, 1);
  }
  var sum = vec4<f32>(0.0);
  var count = 0.0;
  for (var k = -params.radius; k <= params.radius; k = k + 1) {
    let at = pos + step * k;
    if (at.x < 0 || at.y < 0 || at.x >= dims.x || at.y >= dims.y) {
      continue;
    }
    sum = sum + textureLoad(input_tex, at, 0);
    count = count + 1.0;
  }
  textureStore(output_tex, pos, sum / count);
}
