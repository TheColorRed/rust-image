@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> opacity: f32;

// The CPU truncates the multiplied alpha to a byte, so explicitly floor before writing the unorm texture.
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) { return; }
  let pos = vec2<i32>(gid.xy);
  let pixel = textureLoad(input_tex, pos, 0);
  let alpha = floor(round(pixel.a * 255.0) * opacity) / 255.0;
  textureStore(output_tex, pos, vec4<f32>(pixel.rgb, alpha));
}
