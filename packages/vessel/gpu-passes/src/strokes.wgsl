// Draws line segments into a texture that already holds the picture so far, each as a capsule: every pixel near the segment says
// how far it is from the segment, and that distance (against the brush radius and how soft its edge is) says how much of the
// color shows. Only the pixels of a small box around each segment are visited, so a frame costs the length of what is new and not
// the size of the picture. The blend keeps the larger of what is there and what is drawn, so where segments meet there is no seam.
struct Style {
  // The brush color, straight (not multiplied by its alpha), each from 0 to 1; its alpha already holds the brush opacity.
  color: vec4<f32>,
  // Half the width of the brush, in pixels.
  radius: f32,
  // How wide, in pixels, the fade at the edge is. At least one, so the edge is never jagged.
  edge: f32,
  // The size of the texture, in pixels.
  size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> style: Style;

struct Corner {
  @builtin(position) position: vec4<f32>,
  @location(0) origin: vec2<f32>,
  @location(1) target_end: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) index: u32, @location(0) segment: vec4<f32>) -> Corner {
  let origin = segment.xy;
  let target_end = segment.zw;
  let length_of = length(target_end - origin);
  var along = vec2<f32>(1.0, 0.0);
  if (length_of > 0.0) {
    along = (target_end - origin) / length_of;
  }
  let across = vec2<f32>(-along.y, along.x);
  // The box is the segment with the reach of the brush on every side, so the round ends and the fade are inside it.
  let reach = style.radius + style.edge + 1.0;
  var corners = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
  );
  let corner = corners[index];
  let point = origin + along * mix(-reach, length_of + reach, corner.x) + across * mix(-reach, reach, corner.y);
  var out: Corner;
  out.position = vec4<f32>(point.x / style.size.x * 2.0 - 1.0, 1.0 - point.y / style.size.y * 2.0, 0.0, 1.0);
  out.origin = origin;
  out.target_end = target_end;
  return out;
}

@fragment
fn fs(in: Corner) -> @location(0) vec4<f32> {
  let at = in.position.xy;
  let line = in.target_end - in.origin;
  let length_squared = dot(line, line);
  var along = 0.0;
  if (length_squared > 0.0) {
    along = clamp(dot(at - in.origin, line) / length_squared, 0.0, 1.0);
  }
  let nearest = distance(at, in.origin + line * along);
  let coverage = clamp((style.radius - nearest) / style.edge + 0.5, 0.0, 1.0) * style.color.a;
  if (coverage <= 0.0) {
    discard;
  }
  return vec4<f32>(style.color.rgb, coverage);
}
