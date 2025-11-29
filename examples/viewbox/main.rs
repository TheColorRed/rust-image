use abra::drawing::prelude::*;
use abra::prelude::*;

fn main() {
  // Ensure output directory exists
  let _ = std::fs::create_dir_all("out");
  // Define a heart shape in normalized 0-100 coordinate space
  // This is the "abstract" or "viewBox" coordinate system
  let heart = Polygon::new(5);

  // Create three images at different sizes
  let mut image_small = Image::new(120, 120);
  let mut image_medium = Image::new(300, 300);
  let mut image_large = Image::new(600, 600);

  // Render the same heart at all three sizes using viewBox transformation
  let aspect = AspectRatio::meet(); // Preserve aspect ratio, fit within viewport

  // Transform to each viewport size (no explicit ViewBox needed)
  let heart_100 = heart.fit_with_aspect((100.0, 100.0), aspect);
  let heart_300 = heart.fit_with_aspect((300.0, 300.0), aspect);
  let heart_600 = heart.fit_with_aspect((600.0, 600.0), aspect);

  let color = Fill::Gradient(Gradient::hue().reverse());
  // let color = Fill::Solid(Color::from_rgba(255, 50, 100, 255));

  // Areas return from fit/stretch, which are closed shapes (no caps needed, only joins)
  let stroke_100 = heart_100.stroke(5.0, LineJoin::Round);
  let filled_100 = fill(stroke_100, color.clone());
  image_small.set_from(&filled_100, (10, 10));

  let stroke_300 = heart_300.stroke(5.0, LineJoin::Miter);
  let filled_300 = fill(stroke_300, color.clone());
  image_medium.set_from(&filled_300, (0, 0));

  let stroke_600 = heart_600.stroke(5.0, LineJoin::Bevel);
  let filled_600 = fill(stroke_600, color.clone());
  image_large.set_from(&filled_600, (0, 0));

  // Save at different sizes
  image_small.save("out/heart_100.png", None);
  image_medium.save("out/heart_300.png", None);
  image_large.save("out/heart_600.png", None);

  println!("✅ Generated 3 heart images at different sizes:");
  println!("   - heart_100.png (100x100)");
  println!("   - heart_300.png (300x300)");
  println!("   - heart_600.png (600x600)");
  println!();
  println!("All three render the same heart shape defined in 0-100 space,");
  println!("scaled to fit their respective viewport sizes!");

  // Demonstrate different aspect ratio modes
  let mut image_stretch = Image::new(800, 400); // Wide viewport
  let heart_stretched = heart.stretch((800.0, 400.0));
  let stroke_stretched = heart_stretched.stroke(3.0, LineJoin::Miter);
  let filled_stretched = fill(stroke_stretched, Fill::Solid(Color::from_rgba(100, 150, 255, 255)));
  image_stretch.set_from(&filled_stretched, (0, 0));
  image_stretch.save("out/heart_stretched.png", None);

  println!();
  println!("✅ Generated heart_stretched.png (800x400)");
  println!("   Using AspectRatio::none() to stretch and fill the viewport");
}
