use abra::prelude::*;
use abra_ai_core::{AiModel, AiOutput};
use abra_lighting::prelude::*;
use tools::atlas_with;

const MODEL: &str = "packages/ai/depth/models/depth_anything_v2_small.onnx";
const FILE: &str = "assets\\34KK-breasts.webp";

fn main() {
  let file = std::env::args().nth(1).unwrap_or_else(|| FILE.to_string());
  let image = Image::read(&file).expect("Failed to load image");
  let (width, height) = image.dimensions::<u32>();
  let (width, height) = (width as f32, height as f32);

  let AiOutput::Depth(depth) =
    AiModel::load(MODEL).expect("Failed to load the model").run(&image).expect("Failed to estimate the depth")
  else {
    panic!("The depth model answers with a depth map")
  };
  let depth = depth.resized(width as u32, height as u32).values;

  // The same photo under each kind of light.
  let looks = [
    (
      "sun from the left",
      Lighting::new()
        .with_ambient(0.45)
        .with_shadows(0.8)
        .with_light(DirectionalLight::new().with_direction([-1.0, -0.3, 0.6])),
    ),
    (
      "warm lamp",
      Lighting::new().with_ambient(0.3).with_light(
        OmniLight::new()
          .with_position([width * 0.7, height * 0.6, width * 0.15])
          .with_color(Color::from_rgb(255, 191, 115))
          .with_radius(width * 0.3)
          .with_intensity(1.6),
      ),
    ),
    (
      "spotlight, no shadows",
      Lighting::new().with_ambient(0.2).with_light(
        SpotLight::new()
          .with_position([width * 0.5, height * 1.2, width * 0.5])
          .with_target([width * 0.5, height * 0.4, 0.0])
          .with_angle(28.0)
          .with_radius(width * 0.8)
          .with_intensity(1.8),
      ),
    ),
    (
      "spotlight, with shadows",
      Lighting::new().with_ambient(0.2).with_shadows(0.85).with_light(
        SpotLight::new()
          .with_position([width * 0.5, height * 1.2, width * 0.5])
          .with_target([width * 0.5, height * 0.4, 0.0])
          .with_angle(28.0)
          .with_radius(width * 0.8)
          .with_intensity(1.8),
      ),
    ),
    (
      "window",
      Lighting::new().with_ambient(0.35).with_light(
        AreaLight::new()
          .with_position([-width * 0.1, height * 0.3, width * 0.2])
          .with_size(width * 0.2)
          .with_radius(width * 0.6)
          .with_color(Color::from_rgb(204, 230, 255))
          .with_intensity(1.4),
      ),
    ),
  ];

  let mut images = vec![image.clone()];
  for (name, lighting) in looks {
    let mut lit = image.clone();
    let start = std::time::Instant::now();
    lighting.apply(&mut lit, &depth);
    println!("{name}: {:?}", start.elapsed());
    images.push(lit);
  }

  let sheet: Image = atlas_with(images).with_cols(3).into();
  let name = std::path::Path::new(&file).file_stem().unwrap().to_string_lossy();
  sheet.write(&format!("assets/out/lighting.png"), None).expect("Failed to write image");
  println!("✅ Saved assets/out/lighting.png");
}
