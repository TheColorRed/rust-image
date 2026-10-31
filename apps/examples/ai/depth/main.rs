use abra::prelude::*;
use abra_ai_core::{AiModel, AiOutput};
use tools::atlas_with;

const MODEL: &str = "packages/ai/depth/models/depth_anything_v2_small.onnx";
const FILES: [&str; 3] = ["assets/aletta-ocean.jpg", "assets/cat.png", "assets/architecture/central-chambers.jpg"];

fn main() {
  let model = AiModel::load(MODEL).expect("Failed to load the model");
  // Photos named on the command line, or the default set.
  let given: Vec<String> = std::env::args().skip(1).collect();
  let files: Vec<&str> = if given.is_empty() { FILES.to_vec() } else { given.iter().map(String::as_str).collect() };

  for file in files {
    let image = Image::read(file).expect("Failed to load image");
    let AiOutput::Depth(depth) = model.run(&image).expect("Failed to estimate the depth") else {
      panic!("The depth model answers with a depth map")
    };

    // The photo, then its depth (white is near, black is far).
    let sheet: Image = atlas_with(vec![image, depth.to_mask().to_image()]).with_cols(2).into();
    let name = std::path::Path::new(file).file_stem().unwrap().to_string_lossy();
    sheet.write(&format!("assets/out/depth-{name}.png"), None).expect("Failed to write image");
    println!("✅ Saved assets/out/depth-{name}.png");
  }
}
