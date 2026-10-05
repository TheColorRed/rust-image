@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var mask_tex: texture_2d<f32>;
@group(0) @binding(4) var base_tex: texture_2d<f32>;

struct Params {
  blend: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let smoothed = textureLoad(input_tex, pos, 0);
  let original = textureLoad(base_tex, pos, 0);
  let weight = textureLoad(mask_tex, pos, 0).r * params.blend;
  let before = round(original.rgb * 255.0);
  let after = round(smoothed.rgb * 255.0);
  let mixed = clamp(round(before + (after - before) * weight), vec3<f32>(0.0), vec3<f32>(255.0)) / 255.0;
  textureStore(output_tex, pos, vec4<f32>(mixed, original.a));
}