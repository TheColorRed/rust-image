// Replaces each red, green and blue value with its entry in a 256-texel table, and keeps alpha. The table is built on
// the CPU by running the effect's own CPU code over a gray ramp (see `channel_lut_pass` in lut.rs), so the GPU gives
// exactly the pixels the CPU does, with no arithmetic of its own.
// 0 = input, 1 = output, 3 = the table (texel i holds the result for the value i).
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var lut_tex: texture_2d<f32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let p = textureLoad(input_tex, vec2<i32>(i32(gid.x), i32(gid.y)), 0);
  let index = vec3<i32>(round(p.rgb * 255.0));
  let out_rgb = vec3<f32>(
    textureLoad(lut_tex, vec2<i32>(index.r, 0), 0).r,
    textureLoad(lut_tex, vec2<i32>(index.g, 0), 0).g,
    textureLoad(lut_tex, vec2<i32>(index.b, 0), 0).b,
  );
  textureStore(output_tex, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(out_rgb, p.a));
}
