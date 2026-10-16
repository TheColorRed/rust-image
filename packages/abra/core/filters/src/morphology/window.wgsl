struct Params {
  radius: i32,
  // 0 looks along the rows, 1 looks down the columns.
  direction: u32,
  // 1 takes the strongest (brightest) value in the window, 0 the weakest.
  strongest: u32,
  pad0: u32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;
// Where this image sits in the photo it is a view of: pixel `p` is the photo's pixel `origin + (p + 0.5) * scale`. The radius
// is in the photo's pixels, so at another scale the same window is a different number of this image's pixels.
struct Window {
  origin: vec2<f32>,
  scale: f32,
  pad: f32,
  full: vec2<f32>,
  pad2: vec2<f32>,
}
@group(0) @binding(5) var<uniform> window: Window;

// One pass of a grow or shrink: each color channel becomes the strongest or weakest value among the pixels within the radius
// along the row or the column (pixels past the edge count as the nearest edge pixel, which is already in the window, so the edges
// are not eaten). Two passes, one each way, give the square window `Mask::dilated` and `Mask::eroded` use, with the same
// pixels. The alpha is left as it was.
//
// At another scale than the photo's own, the window is `radius / scale` of this image's pixels, which can be more than there is
// time for when the view is deep in the photo, so it looks at no more than 32 pixels each side and spaces them out to cover the
// reach. It is a view and not the saved photo, so it only has to look the same.
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  let pos = vec2<i32>(gid.xy);
  if (pos.x >= dims.x || pos.y >= dims.y) {
    return;
  }

  var step = vec2<i32>(1, 0);
  if (params.direction == 1u) {
    step = vec2<i32>(0, 1);
  }

  var taps = params.radius;
  var spacing = 1.0;
  if (abs(window.scale - 1.0) > 0.0001) {
    let reach = f32(params.radius) / window.scale;
    spacing = max(1.0, reach / 32.0);
    taps = i32(floor(reach / spacing));
  }

  let last = dims - vec2<i32>(1);
  let here = textureLoad(input_tex, pos, 0);
  var best = here.rgb;
  for (var k = -taps; k <= taps; k = k + 1) {
    let offset = i32(round(f32(k) * spacing));
    let sample = textureLoad(input_tex, clamp(pos + step * offset, vec2<i32>(0), last), 0).rgb;
    if (params.strongest == 1u) {
      best = max(best, sample);
    } else {
      best = min(best, sample);
    }
  }
  textureStore(output_tex, pos, vec4<f32>(best, here.a));
}
