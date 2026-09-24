use crate::Color;
use rand::prelude::*;

impl Color {
  pub fn transparent() -> Self { Self { r: 0, g: 0, b: 0, a: 0 } }
  pub fn random() -> Self { let mut rng = rand::rng(); Self { r: rng.random_range(0..255), g: rng.random_range(0..255), b: rng.random_range(0..255), a: 255 } }
  pub fn red() -> Self { Self { r: 255, g: 0, b: 0, a: 255 } }
  pub fn crimson() -> Self { Self { r: 220, g: 20, b: 60, a: 255 } }
  pub fn ruby() -> Self { Self { r: 224, g: 17, b: 95, a: 255 } }
  pub fn pink() -> Self { Self { r: 255, g: 192, b: 203, a: 255 } }
  pub fn magenta() -> Self { Self { r: 255, g: 0, b: 255, a: 255 } }
  pub fn hot_pink() -> Self { Self { r: 255, g: 105, b: 180, a: 255 } }
  pub fn green() -> Self { Self { r: 0, g: 255, b: 0, a: 255 } }
  pub fn lime_green() -> Self { Self { r: 50, g: 205, b: 50, a: 255 } }
  pub fn sea_green() -> Self { Self { r: 46, g: 139, b: 87, a: 255 } }
  pub fn forest_green() -> Self { Self { r: 34, g: 139, b: 34, a: 255 } }
  pub fn blue() -> Self { Self { r: 0, g: 0, b: 255, a: 255 } }
  pub fn royal_blue() -> Self { Self { r: 65, g: 105, b: 225, a: 255 } }
  pub fn sky_blue() -> Self { Self { r: 135, g: 206, b: 235, a: 255 } }
  pub fn navy_blue() -> Self { Self { r: 0, g: 0, b: 128, a: 255 } }
  pub fn yellow() -> Self { Self { r: 255, g: 255, b: 0, a: 255 } }
  pub fn orange() -> Self { Self { r: 255, g: 127, b: 0, a: 255 } }
  pub fn indigo() -> Self { Self { r: 75, g: 0, b: 130, a: 255 } }
  pub fn violet() -> Self { Self { r: 148, g: 0, b: 211, a: 255 } }
  pub fn white() -> Self { Self { r: 255, g: 255, b: 255, a: 255 } }
  pub fn black() -> Self { Self { r: 0, g: 0, b: 0, a: 255 } }
  pub fn gray() -> Self { Self { r: 128, g: 128, b: 128, a: 255 } }
}
