use abra::{
  Color, Fill, Gradient, ImageLoader, Path,
  canvas::{DropShadow, LayerEffects, Stroke},
  plugin::{Plugin, PluginResult},
};
use abra_collage::{CollageOptions, CollagePlugin, CollageStyle};

pub fn main() {
  let image_paths = vec![
    "assets/bikini.jpg",
    "assets/aletta-ocean.jpg",
    "assets/nude/krasivaya.jpg",
    "assets/nude/leanne-crow.jpg",
    "assets/nude/ellen.jpg",
    "assets/nude/tan-girl.jpg",
    "assets/nude/lay-down.jpg",
    "assets/nude/black-hair-and-big-boobs.webp",
    "assets/nude/gravure-idol-black-hair-chest.jpg",
  ];

  let load_start = std::time::Instant::now();

  let loader = ImageLoader::FromGlob(vec!["assets/**/*{boob,tit,chest}*.{jpg}"]).load();
  let img = loader.at(1u8);
  let path = Path::new().line_to((1024 * 3, 1024 * 2)).clone();
  let mut colors = img.unwrap().as_ref().clone();
  let colors = colors.colors().as_slice().unwrap();
  let mut collage_plugin = CollagePlugin::new((1024 * 3, 1024 * 2), loader)
    .with_style(CollageStyle::LayeredGrid(2, 10))
    .with_options(
      CollageOptions::new()
        .with_rotation_range(-15, 15)
        .with_scale_range(1.25, 1.5)
        // .with_background(Color::pink())
        .with_background(
          Gradient::evenly(vec![
            // Color::magenta(),
            Color::mean(colors),
            Color::median(colors),
            Color::mode(colors),
          ])
          .with_direction(path),
        )
        // .with_background(first.unwrap())
        // .with_background(Fill::Image(abra::Image::new_from_path(image_paths[0]).into()))
        // .with_background(Fill::Gradient(Gradient::rainbow()))
        .with_effects(
          LayerEffects::new()
            .with_stroke(Stroke::new().with_fill(Fill::Solid(Color::white())).with_size(50))
            .with_drop_shadow(
              DropShadow::new()
                .with_angle(45.0)
                .with_distance(20.0)
                // .with_opacity(0.2)
                .with_size(100.0),
            ),
        ),
    );

  println!("Images loaded in {:?}", load_start.elapsed());

  let result = collage_plugin.apply().unwrap();
  match result {
    PluginResult::Canvases(canvases) => {
      canvases[0].save("out/collage_result.png", None);
    }
    _ => {}
  }
}
