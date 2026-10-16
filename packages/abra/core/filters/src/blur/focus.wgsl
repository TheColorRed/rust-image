@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var mask_tex: texture_2d<f32>;
@group(0) @binding(4) var base_tex: texture_2d<f32>;

// `input_tex` is the blur result and `base_tex` is the unblurred image. The focus mask chooses between them.
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  let pos = vec2<i32>(gid.xy);
  if (pos.x >= dims.x || pos.y >= dims.y) { return; }
  let strength = textureLoad(mask_tex, pos, 0).r;
  textureStore(output_tex, pos, mix(textureLoad(base_tex, pos, 0), textureLoad(input_tex, pos, 0), strength));
}
