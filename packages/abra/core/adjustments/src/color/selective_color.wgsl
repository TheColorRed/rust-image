@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
// For each of the nine ranges, in the order Reds, Yellows, Greens, Cyans, Blues, Magentas, Whites, Neutrals, Blacks, the change of
// [cyan, magenta, yellow, black]. Then one row whose first number is 1 for a relative change (a share of the ink a pixel has) and
// 0 for an absolute one. See `SelectiveColor` in selective_color.rs: this does the same sums, so the GPU and the CPU agree to a level.
struct Ranges {
  adjustments: array<vec4<f32>, 9>,
  method: vec4<f32>,
}
@group(0) @binding(2) var<uniform> ranges: Ranges;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let p = textureLoad(input_tex, pos, 0);
  // The color as it is in a byte, so the sums start from the same numbers as the CPU's.
  let rgb = round(p.rgb * 255.0) / 255.0;
  let maximum = max(rgb.r, max(rgb.g, rgb.b));
  let minimum = min(rgb.r, min(rgb.g, rgb.b));
  let middle = rgb.r + rgb.g + rgb.b - maximum - minimum;

  // How much the pixel belongs to each range.
  var weight = array<f32, 9>(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
  let most = maximum - middle;
  if (rgb.r >= rgb.g && rgb.r >= rgb.b) {
    weight[0] = most;
  } else if (rgb.g >= rgb.b) {
    weight[2] = most;
  } else {
    weight[4] = most;
  }
  let least = middle - minimum;
  if (rgb.b <= rgb.r && rgb.b <= rgb.g) {
    weight[1] = least;
  } else if (rgb.r <= rgb.g) {
    weight[3] = least;
  } else {
    weight[5] = least;
  }
  weight[6] = minimum;
  weight[7] = clamp(1.0 - (abs(maximum - 0.5) + abs(minimum - 0.5)), 0.0, 1.0);
  weight[8] = 1.0 - maximum;

  let ink = vec3<f32>(1.0) - rgb;
  let black = min(ink.r, min(ink.g, ink.b));
  // The cyan, magenta and yellow that are left once the black is taken out.
  let colors = ink - vec3<f32>(black);
  var result = colors;
  var result_black = black;
  for (var range = 0; range < 9; range = range + 1) {
    if (weight[range] == 0.0) {
      continue;
    }
    let adjustment = ranges.adjustments[range];
    var colored = adjustment.xyz;
    var blacker = adjustment.w;
    if (ranges.method.x > 0.5) {
      colored = colors * adjustment.xyz;
      blacker = black * adjustment.w;
    }
    result = result + colored * weight[range];
    result_black = result_black + blacker * weight[range];
  }
  result_black = clamp(result_black, 0.0, 1.0);
  // Rounded half up to a byte, as the CPU does.
  let out_rgb = floor(clamp(vec3<f32>(1.0) - clamp(max(result, vec3<f32>(0.0)) + vec3<f32>(result_black), vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(0.0), vec3<f32>(1.0)) * 255.0 + vec3<f32>(0.5)) / 255.0;
  textureStore(output_tex, pos, vec4<f32>(out_rgb, p.a));
}
