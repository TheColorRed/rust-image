//! Images whose pixels are made on the GPU, and only when somebody looks at them.
//!
//! [`blended`] and [`generated`] return an [`Image`] that has not been computed. It remembers how it is made, and a chain
//! of blends over one image stays one chain: it runs on the GPU in one go the first time the pixels are read, with nothing
//! copied back to the CPU in between. A preview that can draw the chain itself never reads the pixels at all. The image
//! blended on top is handed to the shader as pixels, so one that is itself deferred, such as a gradient, is made first.
//!
//! The CPU version of every step is kept. It is what runs when there is no GPU, when the GPU is off, when a step has no
//! shader, or when the GPU fails, so asking for the pixels always works. The recipes are only made when the GPU is ready;
//! otherwise these functions just do the work and return a plain image.

use std::any::Any;
use std::sync::Arc;

use primitives::DeferredPixels;

use crate::image::gpu::{GpuAux, GpuPass, Hardware, ready_gpu_provider};
use crate::{BlendMode, Image, blend::blend};

/// Something that makes a whole image from nothing, such as a gradient.
pub trait Generator: Send + Sync + std::fmt::Debug {
  /// The image on the CPU. This is the reference the shader must match.
  fn render(&self, p_width: u32, p_height: u32) -> Image;

  /// Whether [`passes`](Self::passes) has a shader.
  fn has_gpu(&self) -> bool {
    true
  }

  /// The passes that write the image, for an image of the given size. They only use the input for its size. `None` when
  /// there is no shader.
  fn passes(&self, p_width: u32, p_height: u32) -> Option<Vec<GpuPass>>;
}

#[derive(Debug)]
enum Node {
  /// An image made by a generator.
  Generated(Arc<dyn Generator>),
  /// `top` put over `base` with a blend mode.
  Blend {
    base: Image,
    top: Image,
    mode: BlendMode,
    opacity: f32,
  },
}

/// How a deferred image is made. Whoever reads the pixels, or draws them, gets them from here.
#[derive(Debug)]
pub struct Recipe {
  width: u32,
  height: u32,
  node: Node,
}

/// A GPU run that is ready to go: the pixels it starts from, and the passes to run over them.
struct Plan {
  root: Image,
  passes: Vec<Vec<GpuPass>>,
}

fn recipe_of(p_image: &Image) -> Option<&Recipe> {
  p_image.deferred()?.as_any().downcast_ref::<Recipe>()
}

/// Whether recipes are worth making: the GPU is allowed and has started.
fn gpu_ready() -> bool {
  ready_gpu_provider(Hardware::Auto).is_some()
}

/// An image `p_generator` makes. Deferred when the GPU can put it over other images; otherwise made now.
pub fn generated(p_width: u32, p_height: u32, p_generator: Arc<dyn Generator>) -> Image {
  if !p_generator.has_gpu() || !gpu_ready() {
    return p_generator.render(p_width, p_height);
  }
  Image::from_deferred(
    p_width,
    p_height,
    Arc::new(Recipe {
      width: p_width,
      height: p_height,
      node: Node::Generated(p_generator),
    }),
  )
}

/// `p_top` put over `p_base` with `p_mode`, as a new image, with `p_opacity` from 0 to 1 on top of the top's own alpha.
/// Neither image is changed. Deferred when the GPU can do it (same-size images and a built-in mode); otherwise made now,
/// with the same pixels either way.
pub fn blended(p_base: &Image, p_top: &Image, p_mode: BlendMode, p_opacity: f32) -> Image {
  let opacity = p_opacity.clamp(0.0, 1.0);
  let size = p_base.dimensions::<u32>();
  if opacity == 0.0 || p_top.dimensions::<u32>() != size || p_mode.shader_index().is_none() || !gpu_ready() {
    let mut result = p_base.clone();
    blend(p_top).with_mode(p_mode).with_opacity(opacity).apply(&mut result);
    return result;
  }
  // A top made by a generator without a shader would have to be made on the CPU first; that is still correct, so it stays
  // deferred and the plan simply uses the top's pixels.
  Image::from_deferred(
    size.0,
    size.1,
    Arc::new(Recipe {
      width: size.0,
      height: size.1,
      node: Node::Blend {
        base: p_base.clone(),
        top: p_top.clone(),
        mode: p_mode,
        opacity,
      },
    }),
  )
}

/// The passes that blend `p_top` over an image the same size as it. The top is handed to the shader as pixels, as a texture.
/// If it is itself deferred, reading its pixels makes it first.
fn top_passes(p_top: &Image, p_mode: BlendMode, p_opacity: f32) -> Option<Vec<GpuPass>> {
  let mode = p_mode.shader_index()?;
  let (width, height) = p_top.dimensions::<u32>();
  let aux = GpuAux {
    width,
    height,
    rgba: Arc::from(p_top.rgba()),
  };
  Some(vec![
    GpuPass::new(
      concat!(include_str!("../combine/blend_modes.wgsl"), include_str!("../combine/blend.wgsl")),
      [p_opacity.to_le_bytes(), mode.to_le_bytes(), [0; 4], [0; 4]].concat(),
    )
    .with_aux(aux),
  ])
}

/// The GPU run for `p_image`, following its recipes down to the first image that already has pixels.
fn plan(p_image: &Image) -> Option<Plan> {
  match recipe_of(p_image) {
    Some(recipe) => plan_of(recipe),
    // Not a recipe, or one that already made its pixels: it is the pixels the run starts from.
    None => Some(Plan {
      root: p_image.clone(),
      passes: Vec::new(),
    }),
  }
}

/// The GPU run for an image that is still a recipe: the pixels it starts from and the passes over them, in order. For a surface
/// that can draw the run itself, so the picture never goes through the CPU. `None` for an image that has its pixels, and for
/// one with a step that has no shader.
pub fn gpu_run(p_image: &Image) -> Option<(Image, Vec<Vec<GpuPass>>)> {
  recipe_of(p_image)?;
  plan(p_image).map(|plan| (plan.root, plan.passes))
}

impl Recipe {
  /// The pixels from the GPU, or `None` when it is not available or fails.
  fn on_gpu(&self) -> Option<Vec<u8>> {
    let provider = ready_gpu_provider(Hardware::Auto)?;
    let plan = plan_of(self)?;
    (provider.process)(&plan.passes, self.width, self.height, plan.root.rgba()).ok()
  }

  /// The pixels from the CPU. Always works.
  fn on_cpu(&self) -> Vec<u8> {
    match &self.node {
      Node::Generated(generator) => generator.render(self.width, self.height).into_rgba_vec(),
      Node::Blend {
        base,
        top,
        mode,
        opacity,
      } => {
        let mut result = base.clone();
        blend(top).with_mode(*mode).with_opacity(*opacity).apply(&mut result);
        result.into_rgba_vec()
      }
    }
  }
}

/// The GPU run for a recipe.
fn plan_of(p_recipe: &Recipe) -> Option<Plan> {
  match &p_recipe.node {
    Node::Generated(generator) => Some(Plan {
      root: Image::new(p_recipe.width, p_recipe.height),
      passes: vec![generator.passes(p_recipe.width, p_recipe.height)?],
    }),
    Node::Blend {
      base,
      top,
      mode,
      opacity,
    } => {
      let mut plan = plan(base)?;
      plan.passes.push(top_passes(top, *mode, *opacity)?);
      Some(plan)
    }
  }
}

impl DeferredPixels for Recipe {
  fn render(&self) -> Vec<u8> {
    self.on_gpu().unwrap_or_else(|| self.on_cpu())
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Color;
  use crate::image::gpu::{GPU_PROVIDER_LOCK, GpuProvider, clear_gpu_provider, register_gpu_provider};
  use std::sync::atomic::{AtomicUsize, Ordering};

  static RUNS: AtomicUsize = AtomicUsize::new(0);
  static PASSES: AtomicUsize = AtomicUsize::new(0);

  /// A generator that paints one color, and has a "shader" so the fake GPU below can be asked to run it.
  #[derive(Debug)]
  struct Flat(Color);

  impl Generator for Flat {
    fn render(&self, p_width: u32, p_height: u32) -> Image {
      Image::new_from_color(p_width, p_height, self.0)
    }

    fn passes(&self, _p_width: u32, _p_height: u32) -> Option<Vec<GpuPass>> {
      Some(vec![GpuPass::new("flat", [])])
    }
  }

  /// A GPU that answers every run with a marker color, and counts the runs and the passes it was given.
  fn fake_gpu() {
    RUNS.store(0, Ordering::SeqCst);
    PASSES.store(0, Ordering::SeqCst);
    register_gpu_provider(GpuProvider {
      wait_until_ready: || true,
      process: |effects, width, height, _pixels| {
        RUNS.fetch_add(1, Ordering::SeqCst);
        PASSES.fetch_add(effects.len(), Ordering::SeqCst);
        Ok([9u8, 8, 7, 255].repeat((width * height) as usize))
      },
      new_session: || Err("no session".to_string()),
    });
  }

  /// A GPU that is there but fails every run.
  fn broken_gpu() {
    register_gpu_provider(GpuProvider {
      wait_until_ready: || true,
      process: |_effects, _width, _height, _pixels| Err("the GPU went away".to_string()),
      new_session: || Err("no session".to_string()),
    });
  }

  fn photo() -> Image {
    Image::new_from_color(4, 3, Color::from_rgba(200, 100, 50, 255))
  }

  #[test]
  fn without_a_gpu_the_work_is_done_at_once_and_the_image_is_ordinary() {
    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    clear_gpu_provider();
    let top = generated(4, 3, Arc::new(Flat(Color::from_rgba(0, 0, 0, 255))));
    assert!(top.deferred().is_none());
    let result = blended(&photo(), &top, BlendMode::Normal, 0.5);
    assert!(result.deferred().is_none());
    assert_eq!(result.get_pixel(0, 0), Some((100, 50, 25, 255)));
  }

  #[test]
  fn with_a_gpu_a_chain_stays_deferred_and_runs_once_as_one_chain() {
    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    fake_gpu();
    let top = generated(4, 3, Arc::new(Flat(Color::from_rgba(0, 0, 0, 255))));
    let once = blended(&photo(), &top, BlendMode::Normal, 1.0);
    let twice = blended(&once, &top, BlendMode::Normal, 0.5);
    assert!(top.deferred().is_some() && once.deferred().is_some() && twice.deferred().is_some());
    assert_eq!(RUNS.load(Ordering::SeqCst), 0, "nothing runs until the pixels are read");
    assert_eq!(twice.get_pixel(0, 0), Some((9, 8, 7, 255)));
    assert_eq!(twice.get_pixel(1, 1), Some((9, 8, 7, 255)));
    // The top is handed to the shader as pixels, so it is made once (one run, one step), and then the two blends over the
    // photo are the one chain (one run, two steps).
    assert_eq!(RUNS.load(Ordering::SeqCst), 2, "the top once, and one run for the whole chain however often it is read");
    assert_eq!(PASSES.load(Ordering::SeqCst), 3, "one step for the top, and the two blends as two steps of one run");
    clear_gpu_provider();
  }

  #[test]
  fn a_gpu_that_fails_falls_back_to_the_cpu_and_gives_the_same_picture() {
    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    broken_gpu();
    let top = generated(4, 3, Arc::new(Flat(Color::from_rgba(0, 0, 0, 255))));
    let result = blended(&photo(), &top, BlendMode::Normal, 0.5);
    assert!(result.deferred().is_some());
    assert_eq!(result.get_pixel(3, 2), Some((100, 50, 25, 255)));
    clear_gpu_provider();
  }

  #[test]
  fn images_of_different_sizes_or_no_opacity_are_blended_at_once() {
    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    fake_gpu();
    let small = Image::new_from_color(2, 2, Color::from_rgba(0, 0, 0, 255));
    assert!(blended(&photo(), &small, BlendMode::Normal, 1.0).deferred().is_none());
    let flat = Image::new_from_color(4, 3, Color::from_rgba(0, 0, 0, 255));
    assert_eq!(blended(&photo(), &flat, BlendMode::Normal, 0.0).get_pixel(0, 0), Some((200, 100, 50, 255)));
    assert_eq!(RUNS.load(Ordering::SeqCst), 0);
    clear_gpu_provider();
  }
}
