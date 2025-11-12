use abra::{
  canvas::{DropShadowOptions, LayerEffectOptions, StrokeOptions},
  color::{Color, Fill},
  plugin::Plugin,
};
use abra_collage::{CollageOptions, CollagePlugin, CollageStyle};

pub fn main() {
  let start = std::time::Instant::now();
  let images = vec![
    abra::image::Image::new_from_path("assets/bikini.jpg"),
    abra::image::Image::new_from_path("assets/aletta-ocean.jpg"),
    // "assets/nude/malena.jpg",
    abra::image::Image::new_from_path("assets/nude/krasivaya.jpg"),
    abra::image::Image::new_from_path("assets/nude/ellen.jpg"),
    abra::image::Image::new_from_path("assets/nude/tan-girl.jpg"),
    abra::image::Image::new_from_path("assets/nude/lay-down.jpg"),
    abra::image::Image::new_from_path("assets/nude/black-hair-and-big-boobs.webp"),
    abra::image::Image::new_from_path("assets/nude/gravure-idol-black-hair-chest.jpg"),
  ];

  println!("Loaded {} images in {:?}", images.len(), start.elapsed());

  let bg_image = images[0].clone();
  let mut collage_plugin = CollagePlugin::new((1024 * 3, 1024 * 2), images)
    .with_style(CollageStyle::LayeredGrid(6, 5))
    .with_options(
      CollageOptions::new()
        .with_rotation_range(-10.0, 10.0)
        .with_scale_range(1.0, 1.5)
        .with_background(Fill::Image(bg_image.clone()))
        // .with_background(Fill::Gradient(Gradient::rainbow()))
        .with_effects(
          LayerEffectOptions::new()
            .with_stroke(
              StrokeOptions::new()
                .with_fill(Fill::Solid(Color::white()))
                // .with_fill(Fill::Image(bg_image.clone()))
                .with_size(20),
            )
            .with_drop_shadow(
              DropShadowOptions::new()
                .with_angle(45.0)
                .with_distance(10.0)
                .with_size(20.0),
            ),
        ),
    );

  let result = collage_plugin.apply().unwrap();
  match result {
    abra::plugin::PluginResult::Canvases(canvases) => {
      canvases[0].save("out/collage_result.png", None);
    }
    _ => {}
  }
}
