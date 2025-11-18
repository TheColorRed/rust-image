#![allow(unused_imports)]
use abra::{
  canvas::{AddCanvasOptions, Anchor, Canvas, LayerSize, NewLayerOptions},
  color::Color,
  combine::blend::{self, RGBA},
  image::Image,
  transform::{Crop, Resize, TransformAlgorithm},
  utils::fs::WriterOptions,
};

const BOTTOM_IMAGE: &str = "assets/aletta-ocean.jpg";
// const BOTTOM_IMAGE: &str = "assets/boobs.webp";
const TOP_IMAGE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/layers.png";

const CANVAS2_BOTTOM_IMAGE: &str = "assets/34KK-breasts.webp";
const CANVAS2_TOP_IMAGE: &str = "assets/skirt.png";

pub fn main() {
  let canvas_root = Canvas::new("My Canvas Project");

  let canvas1 = Canvas::new("First Canvas")
    .add_layer_from_path(
      "Canvas 1 Bottom Layer",
      BOTTOM_IMAGE,
      Some(NewLayerOptions::new().with_size(LayerSize::Percentage(1.0, None))),
    )
    .add_layer_from_path(
      "Canvas 1 Top Layer",
      TOP_IMAGE,
      Some(
        NewLayerOptions::new()
          .with_size(LayerSize::Percentage(0.5, None))
          // .with_opacity(0.8)
          .with_anchor(Anchor::BottomLeft)
          .with_blend_mode(blend::multiply),
      ),
    );
  // .add_layer_from_path(
  //   "Canvas 1 Top Layer 2",
  //   CANVAS2_BOTTOM_IMAGE,
  //   Some(NewLayerOptions {
  //     size: Some(Size::Cover(None)),
  //     ..Default::default()
  //   }),
  // );

  // let canvas2 = Canvas::new("Second Canvas")
  //   .add_layer_from_path("Canvas 2 Bottom Layer", CANVAS2_BOTTOM_IMAGE, None)
  //   .add_layer_from_path(
  //     "Canvas 2 Top Layer",
  //     CANVAS2_TOP_IMAGE,
  //     Some(
  //       NewLayerOptions::new()
  //         .with_anchor(Anchor::TopLeft)
  //         .with_size(LayerSize::Contain(None)),
  //     ),
  //   );

  // Get layer by index and modify it before adding to root
  if let Some(bg_layer) = canvas1.get_layer_by_index(0) {
    let name = bg_layer.name();
    println!("Modifying layer: {}", name);
    // bg_layer.set_opacity(0.8);
    println!("Modified background layer opacity to 0.8");
  }

  // Get layer by name and modify it before adding to root
  if let Some(top_layer) = canvas1.get_layer_by_name("Canvas 1 Top Layer") {
    let name = top_layer.name();
    println!("Modifying layer: {}", name);
    // top_layer.anchor_to_canvas(Anchor::Center);
    // top_layer.set_blend_mode(blend::screen);
    // top_layer.transform().resize_percentage(50.0);
  }

  canvas_root.add_canvas(canvas1, None);
  // canvas_root.add_canvas(
  //   canvas2,
  //   Some(AddCanvasOptions {
  //     anchor: Some(Anchor::BottomRight),
  //   }),
  // );

  canvas_root.save(OUT_FILE, None);
}
