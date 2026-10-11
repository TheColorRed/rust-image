// Replaces each pixel's color with the gradient's color at the pixel's brightness, and keeps alpha. The brightness is the
// same whole-number sum as grayscale.wgsl, and the gradient is a 256-texel table built on the CPU (see `build_lut` in
// gradient_map.rs), so the GPU gives exactly the pixels the CPU does.
// 0 = input, 1 = output, 3 = the table (texel i holds the color for brightness i).
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var lut_tex: texture_2d<f32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let p = textureLoad(input_tex, pos, 0);
  let rgb = vec3<u32>(round(p.rgb * 255.0));
  let gray = (77u * rgb.r + 150u * rgb.g + 29u * rgb.b + 128u) >> 8u;
  let color = textureLoad(lut_tex, vec2<i32>(i32(gray), 0), 0);
  textureStore(output_tex, pos, vec4<f32>(color.rgb, p.a));
}
