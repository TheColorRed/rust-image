// First pass of the guided filter. Reads the mask picture at binding 0 (its red channel) and the guide photo at binding 3, and
// writes four numbers for each pixel, which are too fine for 8 bits and so go to an `rgba32float` texture:
//   x = the guide's brightness, y = the mask, z = brightness * mask, w = brightness * brightness.
// Brightness is 0.299 red + 0.587 green + 0.114 blue, as the CPU's `guided` uses.
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba32float, write>;
@group(0) @binding(3) var guide_tex: texture_2d<f32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  let pos = vec2<i32>(gid.xy);
  if (pos.x >= dims.x || pos.y >= dims.y) {
    return;
  }
  let guide = textureLoad(guide_tex, pos, 0).rgb;
  let brightness = dot(guide, vec3<f32>(0.299, 0.587, 0.114));
  let mask = textureLoad(input_tex, pos, 0).r;
  textureStore(output_tex, pos, vec4<f32>(brightness, mask, brightness * mask, brightness * brightness));
}
