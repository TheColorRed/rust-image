use abra::{
  ImageLoader,
  canvas::effects::{DropShadow, LayerEffects, Stroke},
  color::{Color, Fill},
  plugin::Plugin,
};
use abra_collage::{CollageOptions, CollagePlugin, CollageStyle};

pub fn main() {
  let image_paths = vec![
    "assets/bikini.jpg",
    "assets/aletta-ocean.jpg",
    // "assets/nude/malena.jpg",
    "assets/nude/krasivaya.jpg",
    "assets/nude/ellen.jpg",
    "assets/nude/tan-girl.jpg",
    "assets/nude/lay-down.jpg",
    "assets/nude/black-hair-and-big-boobs.webp",
    "assets/nude/gravure-idol-black-hair-chest.jpg",
  ];

  println!("Loading {} images in parallel...", image_paths.len());
  let load_start = std::time::Instant::now();

  let loader = ImageLoader::FromPaths(image_paths).load();
  let first = loader.first();
  let mut collage_plugin = CollagePlugin::new((1024 * 3, 1024 * 2), loader)
    .with_style(CollageStyle::LayeredGrid(6, 5))
    .with_options(
      CollageOptions::new()
        .with_rotation_range(-10.0, 10.0)
        .with_scale_range(1.0, 1.5)
        .with_background(Fill::Image(first.unwrap()))
        // .with_background(Fill::Image(abra::Image::new_from_path(image_paths[0]).into()))
        // .with_background(Fill::Gradient(Gradient::rainbow()))
        .with_effects(
          LayerEffects::new()
            .with_stroke(Stroke::new().with_fill(Fill::Solid(Color::white())).with_size(20))
            .with_drop_shadow(
              DropShadow::new()
                .with_angle(45.0)
                .with_distance(10.0)
                .with_opacity(0.8)
                .with_size(20.0),
            ),
        ),
    );

  println!("Images loaded in {:?}", load_start.elapsed());

  let result = collage_plugin.apply().unwrap();
  match result {
    abra::plugin::PluginResult::Canvases(canvases) => {
      canvases[0].save("out/collage_result.png", None);
    }
    _ => {}
  }
}
