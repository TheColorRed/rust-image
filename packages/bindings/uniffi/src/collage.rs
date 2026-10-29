//! `Collage` from the `collage` add-on: a canvas made of several images laid out in a grid, a layered grid or at random.
//!
//! What is not exported, and why:
//! - `Collage::new` takes anything that becomes `LoadedImages`; here it takes the images themselves.
//! - The size is a tuple in the library, so it is an array of two numbers here, and an array of the wrong length panics.

use std::sync::{Arc, Mutex};

use abra::prelude::{LoadedCollection, LoadedImages};
use abra_collage::{Collage as library_collage, CollageOptions as library_options, CollageStyle};

use crate::{Canvas, Fill, Image, LayerEffects};

/// How the images of a collage are laid out.
#[uniffi::remote(Enum)]
pub enum CollageStyle {
  /// Evenly sized cells in the given columns and rows; an image that does not fit its cell is cropped.
  Grid(u32, u32),
  /// Columns and rows where each cell is the size of its image, so neighbours overlap.
  LayeredGrid(u32, u32),
  /// The given number of images at random positions.
  Random(u32),
}

/// The rotation, scale, background and layer effects of a collage.
#[derive(Clone, uniffi::Object)]
pub struct CollageOptions(library_options<'static>);

#[uniffi::export]
impl CollageOptions {
  #[uniffi::constructor]
  pub fn new() -> Self {
    Self(library_options::new())
  }

  /// Sets the range, in degrees, that each image is rotated within. Positive values rotate clockwise.
  pub fn with_rotation_range(&self, min: f64, max: f64) -> Arc<CollageOptions> {
    Arc::new(Self(self.0.clone().with_rotation_range(min, max)))
  }

  /// Sets the range that each image is scaled within. Values over 1 enlarge and values under 1 shrink.
  pub fn with_scale_range(&self, min: f64, max: f64) -> Arc<CollageOptions> {
    Arc::new(Self(self.0.clone().with_scale_range(min, max)))
  }

  /// Sets what the collage is drawn on.
  pub fn with_background(&self, background: Arc<Fill>) -> Arc<CollageOptions> {
    Arc::new(Self(self.0.clone().with_background(background.0.clone())))
  }

  /// Sets the effects of every image in the collage.
  pub fn with_effects(&self, effects: Arc<LayerEffects>) -> Arc<CollageOptions> {
    Arc::new(Self(self.0.clone().with_effects(effects.0.clone())))
  }
}

/// A collage of images.
#[derive(uniffi::Object)]
pub struct Collage(Mutex<Option<library_collage<'static>>>);

impl Collage {
  /// Changes the collage the way its builder functions do, which take it by value.
  fn change(&self, change: impl FnOnce(library_collage<'static>) -> library_collage<'static>) {
    let mut collage = self.0.lock().unwrap();
    *collage = collage.take().map(change);
  }
}

#[uniffi::export]
impl Collage {
  /// A collage of `size` (width and height) made from `images`.
  #[uniffi::constructor]
  pub fn new(size: Vec<u32>, images: Vec<Arc<Image>>) -> Self {
    let [width, height] = size[..] else { panic!("a collage size is a width and a height") };
    let mut loaded = LoadedImages::default();
    for image in images {
      loaded.add(Arc::new(image.core()));
    }
    Self(Mutex::new(Some(library_collage::new((width, height), loaded))))
  }

  /// Sets how the images are laid out. Returns the same collage.
  pub fn with_style(self: Arc<Self>, style: CollageStyle) -> Arc<Collage> {
    self.change(|collage| collage.with_style(style));
    self
  }

  /// Sets the rotation, scale, background and effects. Returns the same collage.
  pub fn with_options(self: Arc<Self>, options: Arc<CollageOptions>) -> Arc<Collage> {
    self.change(|collage| collage.with_options(options.0.clone()));
    self
  }

  /// Makes the collage in the style that was chosen and returns it as a canvas.
  pub fn build(&self) -> Arc<Canvas> {
    let mut collage = self.0.lock().unwrap();
    Arc::new(Canvas(collage.as_mut().expect("the collage is being changed").build()))
  }
}
