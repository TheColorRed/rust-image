//! Filters module contains all the filters that can be applied to an image.

pub mod blur;
pub mod distort;
pub mod edges;
pub mod noise;
pub mod repair;
pub mod sharpen;
pub mod smooth;
pub mod sobel;

mod kernel;

pub use options::Effect;

pub(crate) mod common {
  pub use abra_core::{Image, ImageRef};
  pub use options::Effect;
  pub use options::ApplyOptions;
  pub use options::Options;
  pub use rayon::prelude::*;
}
