@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;

// @group(0) @binding(2) var<uniform> radius: f32;

fn generateGaussianKernel(radius: f32) -> array<array<f32, 32>, 32> {
  var kernel: array<array<f32, 32>, 32>;
  let sigma: f32 = radius;
  let twoSigmaSq: f32 = 2.0 * sigma * sigma;
  let piSigma: f32 = 3.141592653589793 * twoSigmaSq;
  let radiusInt: i32 = i32(radius);

  for (var i = -radiusInt; i <= radiusInt; i = i + 1) {
    for (var j = -radiusInt; j <= radiusInt; j = j + 1) {
      let distance: f32 = f32(i * i + j * j);
      kernel[i + radiusInt][j + radiusInt] = exp(-distance / twoSigmaSq) / piSigma;
    }
  }

  return kernel;
}

fn applyGaussianBlur(x: i32, y: i32, radius: f32, src: texture_2d<f32>) {
  var kernel: array<array<f32, 32>, 32> = generateGaussianKernel(radius);
  var color: vec4<f32> = textureLoad(src, vec2<i32>(x, y), 0);
  var textureWidth = textureDimensions(src).x;
  var textureHeight = textureDimensions(src).y;

  var new_color: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, color.a);
  var total_weight: f32 = 0.0;

  for (var ky = -i32(radius); ky <= i32(radius); ky = ky + 1) {
    for (var kx = -i32(radius); kx <= i32(radius); kx = kx + 1) {
      var px = x + kx;
      var py = y + ky;

      // Boundary check
      if (px >= 0 && px < i32(textureWidth) && py >= 0 && py < i32(textureHeight)) {
        var weight = kernel[ky + i32(radius)][kx + i32(radius)] * 2.0;
        total_weight = total_weight + weight;
        var neighbor_color: vec4<f32> = textureLoad(src, vec2<i32>(px, py), 0);

        var accumulated_color: vec3<f32> = new_color.rgb + neighbor_color.rgb * weight;
        new_color = vec4<f32>(accumulated_color, new_color.a);
      }
    }
  }

  new_color = vec4<f32>(new_color.rgb / total_weight, new_color.a);
  textureStore(output_texture, vec2<i32>(x, y), new_color);
}

fn generateGaussianKernel1D(radius: f32) -> array<f32, 32> {
  var kernel: array<f32, 32>;
  let sigma: f32 = radius / 2.0;
  let twoSigmaSq: f32 = 2.0 * sigma * sigma;
  let piSigma: f32 = 3.141592653589793 * twoSigmaSq;
  let radiusInt: i32 = i32(radius);

  for (var i = -radiusInt; i <= radiusInt; i = i + 1) {
    let distance: f32 = f32(i * i);
    kernel[i + radiusInt] = exp(-distance / twoSigmaSq) / piSigma;
  }

  return kernel;
}

fn applyGaussianBlur1D(x: i32, y: i32, radius: f32, src: texture_2d<f32>, horizontal: bool) -> vec4<f32> {
  var kernel: array<f32, 32> = generateGaussianKernel1D(radius);
  var color: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);
  var textureWidth = textureDimensions(src).x;
  var textureHeight = textureDimensions(src).y;
  var total_weight: f32 = 0.0;

  for (var k = -i32(radius); k <= i32(radius); k = k + 1) {
    var px = x + select(k, 0, horizontal);
    var py = y + select(0, k, horizontal);

    // Boundary check
    if (px >= 0 && px < i32(textureWidth) && py >= 0 && py < i32(textureHeight)) {
      var sample: vec4<f32> = textureLoad(src, vec2<i32>(px, py), 0);
      var weight: f32 = kernel[k + i32(radius)];
      color += sample * weight;
      total_weight += weight;
    }
  }

  return color / total_weight;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let radius: f32 = 20.0;
  let x = i32(global_id.x);
  let y = i32(global_id.y);

  // Apply horizontal blur
  let horizontal: vec4<f32> = applyGaussianBlur1D(x, y, radius, input_texture, true);

  // Apply vertical blur on the intermediate result
  let vertical: vec4<f32> = applyGaussianBlur1D(x, y, radius, input_texture, false);

  // Store the final result
  textureStore(output_texture, vec2<i32>(x, y), vertical + horizontal);
}
