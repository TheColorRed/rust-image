@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

// Passes 2, 3, 5 and 6 of the GPU version of `SmoothSkin` (see `passes` in skin.rs). Blurs the alpha channel, where the
// mask is carried, with a box one pixel tall or wide. Two passes, across and then down, make the square box blur that
// `box_blur_f32_inplace` does on the CPU: values past the border count as 0 and the sum is divided by the full box.
// The color channels are passed along unchanged.
//
// The uniform is two u32s, 8 bytes:
//   radius: half the box's side, in pixels. The box is (2 * radius + 1) pixels wide.
//   across: 1 to blur along the row, 0 to blur down the column.
struct Params {
  radius: u32,
  across: u32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  if (gid.x >= u32(dims.x) || gid.y >= u32(dims.y)) {
    return;
  }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let r = i32(params.radius);
  let direction = select(vec2<i32>(0, 1), vec2<i32>(1, 0), params.across == 1u);

  var sum = 0.0;
  for (var k = -r; k <= r; k++) {
    let at = pos + direction * k;
    if (at.x >= 0 && at.y >= 0 && at.x < dims.x && at.y < dims.y) {
      sum += textureLoad(input_tex, at, 0).a;
    }
  }
  let center = textureLoad(input_tex, pos, 0);
  textureStore(output_tex, pos, vec4<f32>(center.rgb, sum / f32(2 * r + 1)));
}
