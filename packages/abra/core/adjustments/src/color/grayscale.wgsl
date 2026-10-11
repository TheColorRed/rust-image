@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) { return; }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let p = textureLoad(input_tex, pos, 0);
  let rgb = vec3<u32>(round(p.rgb * 255.0));
  let gray = (77u * rgb.r + 150u * rgb.g + 29u * rgb.b + 128u) >> 8u;
  let g = f32(gray) / 255.0;
  textureStore(output_tex, pos, vec4<f32>(g, g, g, p.a));
}