// Draws a particle system, matching the CPU drawing in `lib.rs`. This file is joined after `blend_modes.wgsl` at compile time, which
// holds `composite` and every blend mode.
//
// The particles are made on the CPU, one after another, since they share a random stream, and arrive in the texture at binding 3 as
// a list of 32-bit words, one in each texel: the four bytes of a texel are a word, with the first byte the lowest. A float is its own
// bits. The words are, in order, the particles (16 each), the trails (8 each), the points the trails go through (8 each), the pixels
// of the textures, and then the image cut into squares of `tile_size` pixels, with the items that touch each square in the order
// they are drawn. A pixel only looks at the items of its own square, so a frame does not cost every pixel every particle.
//
// An item is drawn over what is under it the way the CPU does it, a step at a time, with the pixel held as whole numbers 0-255 after
// each step, so a dot matches the CPU to the level.
struct Params {
  aux_width: u32,
  particles_at: u32,
  trails_at: u32,
  vertices_at: u32,
  tiles_at: u32,
  list_at: u32,
  tile_size: u32,
  tiles_x: u32,
  pad0: u32,
  pad1: u32,
  pad2: u32,
  pad3: u32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var aux_tex: texture_2d<f32>;

const ADDITIVE: u32 = 65535u;
const FROM_PIXEL: u32 = 65536u;
const NO_SPRITE: u32 = 4294967295u;
const TRAIL_ITEM: u32 = 2147483648u;

// The word at `i`.
fn word(i: u32) -> u32 {
  let texel = textureLoad(aux_tex, vec2<i32>(i32(i % params.aux_width), i32(i / params.aux_width)), 0);
  let bytes = vec4<u32>(round(texel * 255.0));
  return bytes.x | (bytes.y << 8u) | (bytes.z << 16u) | (bytes.w << 24u);
}

fn real(i: u32) -> f32 {
  return bitcast<f32>(word(i));
}

// `src` (red, green and blue, 0-255) over `dst` (RGBA, 0-255) with the particle's blend: the CPU's `Blend::composite` with a source
// that is fully opaque and a coverage that says how much of it shows. Normal is source-over; the others mix the blend color in by
// the coverage, and additive adds the light scaled by it.
fn blend_over(dst: vec4<f32>, src: vec3<f32>, coverage: f32, code: u32) -> vec4<f32> {
  if (code == 0u) {
    return composite(dst, vec4<f32>(src, 255.0), coverage, 0u);
  }
  let out_alpha = coverage + dst.a / 255.0 * (1.0 - coverage);
  var rgb: vec3<f32>;
  if (code == ADDITIVE) {
    rgb = dst.rgb + src * coverage;
  } else {
    let blended = vec3<f32>(blend_rgb(code, vec3<u32>(dst.rgb), vec3<u32>(src)));
    rgb = dst.rgb + (blended - dst.rgb) * coverage;
  }
  return vec4<f32>(clamp(floor(rgb + 0.5), vec3<f32>(0.0), vec3<f32>(255.0)), clamp(floor(out_alpha * 255.0 + 0.5), 0.0, 255.0));
}

// One pixel of a texture: red, green, blue and alpha as 0-255.
fn sprite_pixel(base: u32, width: u32, x: u32, y: u32) -> vec4<f32> {
  let packed = word(base + y * width + x);
  return vec4<f32>(f32(packed & 255u), f32((packed >> 8u) & 255u), f32((packed >> 16u) & 255u), f32(packed >> 24u));
}

// The color of a texture at `u`, `v` (0 to 1 across it), mixed from the four pixels around that spot, as the CPU's `texel`.
fn sprite_texel(base: u32, width: u32, height: u32, u: f32, v: f32) -> vec4<f32> {
  let x = clamp(u * f32(width) - 0.5, 0.0, f32(width - 1u));
  let y = clamp(v * f32(height) - 0.5, 0.0, f32(height - 1u));
  let x0 = u32(floor(x));
  let y0 = u32(floor(y));
  let x1 = min(x0 + 1u, width - 1u);
  let y1 = min(y0 + 1u, height - 1u);
  let tx = x - f32(x0);
  let ty = y - f32(y0);
  let a = sprite_pixel(base, width, x0, y0);
  let b = sprite_pixel(base, width, x1, y0);
  let c = sprite_pixel(base, width, x0, y1);
  let d = sprite_pixel(base, width, x1, y1);
  let top = a + (b - a) * tx;
  let bottom = c + (d - c) * tx;
  return top + (bottom - top) * ty;
}

// A dot or a textured particle: 16 words, `x y size opacity red green blue sin cos reach blend sprite sprite_width sprite_height
// birth_x birth_y`. The blend word holds the blend in its low 16 bits and `FROM_PIXEL` when the color is the picture's, at the
// birth point, and not the one in the record.
fn shade_particle(at: u32, pos: vec2<i32>, dims: vec2<i32>, dst: vec4<f32>) -> vec4<f32> {
  let x = real(at);
  let y = real(at + 1u);
  let reach = bitcast<i32>(word(at + 9u));
  if (abs(pos.x - i32(x)) > reach || abs(pos.y - i32(y)) > reach) {
    return dst;
  }
  let size = real(at + 2u);
  let opacity = real(at + 3u);
  let flags = word(at + 10u);
  var color = vec3<f32>(real(at + 4u), real(at + 5u), real(at + 6u));
  if ((flags & FROM_PIXEL) != 0u) {
    let born = clamp(vec2<i32>(i32(real(at + 14u)), i32(real(at + 15u))), vec2<i32>(0), dims - vec2<i32>(1));
    color = round(textureLoad(input_tex, born, 0).rgb * 255.0);
  }
  let dx = f32(pos.x) + 0.5 - x;
  let dy = f32(pos.y) + 0.5 - y;
  var coverage = 0.0;
  let sprite = word(at + 11u);
  if (sprite == NO_SPRITE) {
    coverage = clamp((size - sqrt(dx * dx + dy * dy)) / max(size, 1.0) + 0.5, 0.0, 1.0) * opacity;
  } else {
    let sin_turn = real(at + 7u);
    let cos_turn = real(at + 8u);
    let width = word(at + 12u);
    let height = word(at + 13u);
    // Turn the pixel back by the particle's rotation to find where it falls on the texture.
    let u = dx * cos_turn + dy * sin_turn;
    let v = -dx * sin_turn + dy * cos_turn;
    let longest = f32(max(width, height));
    let half_width = size * f32(width) / longest;
    let half_height = size * f32(height) / longest;
    let tu = u / (2.0 * half_width) + 0.5;
    let tv = v / (2.0 * half_height) + 0.5;
    if (tu < 0.0 || tu >= 1.0 || tv < 0.0 || tv >= 1.0) {
      return dst;
    }
    let texel = sprite_texel(sprite, width, height, tu, tv);
    color = floor(texel.rgb * color / 255.0);
    coverage = texel.a / 255.0 * opacity;
  }
  if (coverage <= 0.0) {
    return dst;
  }
  return blend_over(dst, color, coverage, flags & 65535u);
}

// How much of a stroke shows at `distance` from its middle line, where its half width is `reach`, for the brush's hardness: the same
// falloff as the drawing tool's stroke brush.
fn stroke_falloff(distance: f32, reach: f32, hardness: f32) -> f32 {
  if (distance >= reach) {
    return 0.0;
  }
  let across = distance / reach;
  if (hardness < 0.5) {
    let mixture = hardness * 2.0;
    return max((1.0 - across * across) * (1.0 - mixture) + (1.0 - across) * mixture, 0.0);
  }
  let mixture = (hardness - 0.5) * 2.0;
  var hard = 0.0;
  if (across < 0.5) {
    hard = 1.0;
  }
  return max((1.0 - across) * (1.0 - mixture) + hard * mixture, 0.0);
}

// A trail, a line through points that has a width, an opacity and a color at each: 8 words, `first_point point_count hardness blend`
// and four unused. A point is `x y half_width opacity red green blue time`. The pixel finds the closest place on the line and reads
// the line's width, opacity and color there, mixed between the two points either side of it.
fn shade_trail(at: u32, pos: vec2<i32>, dst: vec4<f32>) -> vec4<f32> {
  let first = word(at);
  let count = word(at + 1u);
  let hardness = real(at + 2u);
  let code = word(at + 3u);
  let here = vec2<f32>(f32(pos.x) + 0.5, f32(pos.y) + 0.5);
  var best = 1.0e30;
  var best_point = 0u;
  var best_along = 0.0;
  for (var i = 0u; i + 1u < count; i = i + 1u) {
    let a = params.vertices_at + (first + i) * 8u;
    let b = a + 8u;
    let origin = vec2<f32>(real(a), real(a + 1u));
    let line = vec2<f32>(real(b), real(b + 1u)) - origin;
    let length_squared = dot(line, line);
    var along = 0.0;
    if (length_squared > 0.0) {
      along = clamp(dot(here - origin, line) / length_squared, 0.0, 1.0);
    }
    let offset = here - (origin + line * along);
    let distance_squared = dot(offset, offset);
    if (distance_squared < best) {
      best = distance_squared;
      best_point = a;
      best_along = along;
    }
  }
  let next = best_point + 8u;
  let half_width = mix(real(best_point + 2u), real(next + 2u), best_along);
  let distance = sqrt(best);
  if (half_width < 0.01 || distance >= half_width + 0.5) {
    return dst;
  }
  let opacity = mix(real(best_point + 3u), real(next + 3u), best_along);
  let color = vec3<f32>(
    mix(real(best_point + 4u), real(next + 4u), best_along),
    mix(real(best_point + 5u), real(next + 5u), best_along),
    mix(real(best_point + 6u), real(next + 6u), best_along),
  );
  // The line's own falloff, and a one pixel fade at its edge so a thin line is not jagged.
  let edge = clamp(half_width + 0.5 - distance, 0.0, 1.0);
  let coverage = stroke_falloff(distance, half_width, hardness) * opacity * edge;
  if (coverage <= 0.0) {
    return dst;
  }
  return blend_over(dst, color, coverage, code);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let size = textureDimensions(input_tex);
  if (gid.x >= size.x || gid.y >= size.y) {
    return;
  }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let dims = vec2<i32>(i32(size.x), i32(size.y));
  var dst = round(textureLoad(input_tex, pos, 0) * 255.0);
  let tile = (gid.y / params.tile_size) * params.tiles_x + gid.x / params.tile_size;
  let end = word(params.tiles_at + tile + 1u);
  for (var k = word(params.tiles_at + tile); k < end; k = k + 1u) {
    let item = word(params.list_at + k);
    if ((item & TRAIL_ITEM) != 0u) {
      dst = shade_trail(params.trails_at + (item & 2147483647u) * 8u, pos, dst);
    } else {
      dst = shade_particle(params.particles_at + item * 16u, pos, dims, dst);
    }
  }
  textureStore(output_tex, pos, dst / 255.0);
}
