// Blends the texture at binding 3 over the input with the mode in the uniform. This file is joined after
// `blend_modes.wgsl` at compile time, which holds `composite` and every mode.
//
// `params.x` is the opacity from 0 to 1, on top of the top texture's own alpha, and `params.y` is the mode, its index in
// `BlendMode::ALL`.
struct Params {
  opacity: f32,
  mode: u32,
  pad0: u32,
  pad1: u32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var top_tex: texture_2d<f32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let dst = round(textureLoad(input_tex, pos, 0) * 255.0);
  let src = round(textureLoad(top_tex, pos, 0) * 255.0);
  textureStore(output_tex, pos, composite(dst, src, params.opacity, params.mode) / 255.0);
}
