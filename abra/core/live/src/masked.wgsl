// Mixes an effect's result with the image the effect started from, by a per-pixel weight.
// 0 = input after the effect, 1 = output, 3 = weights (red channel), 4 = the image before the effect.
@group(0) @binding(0) var after_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var weights_tex: texture_2d<f32>;
@group(0) @binding(4) var base_tex: texture_2d<f32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(after_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let xy = vec2<i32>(i32(gid.x), i32(gid.y));
  let weight = textureLoad(weights_tex, xy, 0).r;
  let before = textureLoad(base_tex, xy, 0);
  let after = textureLoad(after_tex, xy, 0);
  textureStore(output_tex, xy, mix(before, after, weight));
}
