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

use crate::image::gpu::{CpuProcessor, GpuAux, GpuPass, Hardware, ready_gpu_provider};
use crate::image::image_ext::ImageExt;
use crate::image::levels::{copy_clamped, level_size};
use crate::{BlendMode, Channels, Image, blend::blend};

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

/// An effect kept in a recipe, so the CPU can still run it when the GPU cannot.
struct Step(Arc<dyn CpuProcessor>);

impl std::fmt::Debug for Step {
  fn fmt(&self, p_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    p_formatter.write_str("Step")
  }
}

#[derive(Debug)]
enum Node {
  /// An image made by a generator.
  Generated(Arc<dyn Generator>),
  /// `base` with an effect applied to all of it.
  Effect { base: Image, effect: Step },
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

/// `p_base` with `p_effect` applied to all of it, as a new image that is not computed until somebody looks at it. The effect's
/// CPU version is kept, so asking for the pixels always works, and so does asking for just a part of them
/// ([`region`](crate::image::image_ext::ImageExt::region)): an effect with a shader is drawn by whoever can draw it, and one
/// without is run on the part that is asked for, with the pixels around it that it reads.
pub fn effect_applied(p_base: &Image, p_effect: Arc<dyn CpuProcessor>) -> Image {
  let (width, height) = p_base.dimensions::<u32>();
  Image::from_deferred(
    width,
    height,
    Arc::new(Recipe {
      width,
      height,
      node: Node::Effect {
        base: p_base.clone(),
        effect: Step(p_effect),
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
    .with_window()
    .with_aux(aux),
  ])
}

/// The GPU run for `p_image`, following its recipes down to the first image that already has pixels.
fn plan(p_image: &Image) -> Option<Plan> {
  match recipe_of(p_image) {
    // An effect with no shader is where the run starts: it is made by the CPU, a part at a time, and the passes go over it.
    Some(Recipe { node: Node::Effect { effect, .. }, .. }) if effect.0.gpu().is_none() => Some(Plan {
      root: p_image.clone(),
      passes: Vec::new(),
    }),
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
      Node::Effect { base, effect } => {
        let mut result = base.clone();
        effect.0.process(&mut result);
        result.into_rgba_vec()
      }
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
    Node::Effect { base, effect } => {
      let mut plan = plan(base)?;
      let passes = effect.0.gpu()?.passes(p_recipe.width, p_recipe.height);
      if !passes.is_empty() {
        plan.passes.push(passes);
      }
      Some(plan)
    }
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

  fn region(&self, p_level: u32, p_left: i32, p_top: i32, p_width: u32, p_height: u32) -> Option<Vec<u8>> {
    // Only an effect run by the CPU can be made a part at a time here; the rest of the recipes are made whole.
    let Node::Effect { base, effect } = &self.node else { return None };
    if effect.0.positional() || p_width == 0 || p_height == 0 {
      return None;
    }
    // At another level the effect is worked out again for that size, and one that cannot is made whole.
    let scaled = if p_level == 0 { None } else { Some(effect.0.at_level(p_level)?) };
    let effect = scaled.as_ref().unwrap_or(&effect.0);
    // Past the edge of the picture is the edge of what the effect made, so the effect runs on the part inside the picture
    // (at least a column and a row of it) and the edge is repeated outward after.
    let (level_width, level_height) = level_size(self.width, self.height, p_level);
    let first_column = p_left.clamp(0, level_width as i32 - 1);
    let first_row = p_top.clamp(0, level_height as i32 - 1);
    let last_column = (p_left + p_width as i32 - 1).clamp(first_column, level_width as i32 - 1);
    let last_row = (p_top + p_height as i32 - 1).clamp(first_row, level_height as i32 - 1);
    let (inside_width, inside_height) = ((last_column - first_column + 1) as u32, (last_row - first_row + 1) as u32);
    // The pixels around it that the effect reads, so the edge of the part is what it would be in the whole picture.
    let pad = effect.padding().max(0) as u32;
    let (width, height) = (inside_width + 2 * pad, inside_height + 2 * pad);
    let around = base.region(p_level, first_column - pad as i32, first_row - pad as i32, width, height);
    let mut part = Image::new_from_pixels(width, height, around, Channels::RGBA);
    effect.process(&mut part);
    let made = copy_clamped(part.rgba(), width, height, pad as i32, pad as i32, inside_width, inside_height);
    Some(copy_clamped(&made, inside_width, inside_height, p_left - first_column, p_top - first_row, p_width, p_height))
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Color;
  use crate::image::effect::{Effect, Options, deferred};
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
    assert_eq!(
      RUNS.load(Ordering::SeqCst),
      2,
      "the top once, and one run for the whole chain however often it is read"
    );
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

  /// An effect that sets the red channel to 99, with a "shader" for the fake GPU.
  #[derive(Clone)]
  struct Red99 {
    options: Options,
  }

  impl Effect for Red99 {
    fn options(&self) -> &Options {
      &self.options
    }

    fn options_mut(&mut self) -> &mut Options {
      &mut self.options
    }

    fn cpu_processor(&self, p_image: &mut Image) {
      let mut pixels = p_image.to_rgba_vec();
      pixels.chunks_exact_mut(4).for_each(|pixel| pixel[0] = 99);
      p_image.set_rgba(pixels);
    }

    fn gpu_processor(&self) -> Option<&dyn crate::image::gpu::GpuProcessor> {
      Some(self)
    }
  }

  impl crate::image::gpu::GpuProcessor for Red99 {
    fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
      vec![GpuPass::new("red99", [])]
    }
  }

  fn red_99() -> Red99 {
    Red99 { options: None }
  }

  #[test]
  fn an_effect_applied_to_a_deferred_target_waits_and_runs_once_with_the_blends() {
    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    fake_gpu();
    let mut image = photo();
    red_99().apply(deferred(&mut image));
    assert!(image.deferred().is_some());
    let top = Image::new_from_color(4, 3, Color::from_rgba(0, 0, 0, 255));
    let result = blended(&image, &top, BlendMode::Normal, 0.5);
    assert_eq!(RUNS.load(Ordering::SeqCst), 0, "nothing runs until the pixels are read");
    let (root, passes) = gpu_run(&result).expect("a run");
    assert_eq!(root.get_pixel(0, 0), Some((200, 100, 50, 255)), "it starts from the photo's own pixels");
    assert_eq!(passes.len(), 2, "the effect, then the blend, as one run");
    assert_eq!(result.get_pixel(0, 0), Some((9, 8, 7, 255)));
    assert_eq!(PASSES.load(Ordering::SeqCst), 2);
    clear_gpu_provider();
  }

  #[test]
  fn a_deferred_effect_gives_the_cpu_pixels_when_the_gpu_fails_and_when_there_is_none() {
    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    broken_gpu();
    let mut image = photo();
    red_99().apply(deferred(&mut image));
    assert!(image.deferred().is_some());
    assert_eq!(image.get_pixel(1, 1), Some((99, 100, 50, 255)));
    clear_gpu_provider();
    let mut plain = photo();
    red_99().apply(deferred(&mut plain));
    assert!(plain.deferred().is_some(), "with no GPU the effect still waits, and the CPU does it when it is looked at");
    assert_eq!(plain.get_pixel(1, 1), Some((99, 100, 50, 255)));
  }

  #[test]
  fn an_effect_with_an_area_or_set_to_the_cpu_is_applied_at_once() {
    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    fake_gpu();
    let mut in_an_area = photo();
    red_99().with_area(crate::Area::rect((0.0, 0.0), (2.0, 2.0))).apply(deferred(&mut in_an_area));
    assert!(in_an_area.deferred().is_none());
    let mut on_the_cpu = photo();
    red_99().with_hardware(Hardware::Cpu).apply(deferred(&mut on_the_cpu));
    assert!(on_the_cpu.deferred().is_none());
    assert_eq!(on_the_cpu.get_pixel(0, 0), Some((99, 100, 50, 255)));
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
  /// An effect with no shader that sets each pixel to the one to its right (the last column keeps its own), so it reads one
  /// pixel around the one it changes and needs a pixel of padding at the edge of a part. A level at half the size shifts by
  /// the same one pixel there, which is what makes a test of the levels easy to follow.
  #[derive(Clone)]
  struct ShiftLeft {
    options: Options,
  }

  impl Effect for ShiftLeft {
    fn options(&self) -> &Options {
      &self.options
    }

    fn options_mut(&mut self) -> &mut Options {
      &mut self.options
    }

    fn padding(&self) -> i32 {
      1
    }

    fn cpu_processor(&self, p_image: &mut Image) {
      let (width, height) = p_image.dimensions::<usize>();
      let before = p_image.to_rgba_vec();
      let mut after = before.clone();
      for y in 0..height {
        for x in 0..width {
          let from = (y * width + (x + 1).min(width - 1)) * 4;
          after[(y * width + x) * 4..(y * width + x) * 4 + 4].copy_from_slice(&before[from..from + 4]);
        }
      }
      p_image.set_rgba(after);
    }

    fn for_level(&self, _p_level: u32) -> Option<Arc<dyn CpuProcessor>> {
      Some(Arc::new(self.clone()))
    }
  }

  /// An effect that says nothing about how it works at another size.
  #[derive(Clone)]
  struct Unscaled(ShiftLeft);

  impl Effect for Unscaled {
    fn options(&self) -> &Options {
      self.0.options()
    }

    fn options_mut(&mut self) -> &mut Options {
      self.0.options_mut()
    }

    fn padding(&self) -> i32 {
      1
    }

    fn cpu_processor(&self, p_image: &mut Image) {
      self.0.cpu_processor(p_image);
    }
  }

  /// A `p_width` x `p_height` image whose red is its column and green its row.
  fn ramp(p_width: u32, p_height: u32) -> Image {
    let mut pixels = Vec::new();
    for y in 0..p_height {
      for x in 0..p_width {
        pixels.extend_from_slice(&[x as u8, y as u8, 100, 255]);
      }
    }
    Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA)
  }

  #[test]
  fn a_part_of_a_deferred_cpu_effect_is_what_the_whole_has_there_and_nothing_else_is_made() {
    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    clear_gpu_provider();
    let base = ramp(8, 6);
    let mut whole = base.clone();
    ShiftLeft { options: None }.apply_to_image(&mut whole);
    let mut later = base.clone();
    ShiftLeft { options: None }.apply(deferred(&mut later));
    assert!(later.deferred().is_some());
    // Inside, across the right edge (where the whole keeps the last column), and reaching past the picture.
    for (left, top, width, height) in [(2, 1, 3, 2), (5, 0, 3, 3), (6, 4, 4, 4), (-2, -2, 4, 4)] {
      assert_eq!(
        later.region(0, left, top, width, height),
        whole.region(0, left, top, width, height),
        "the part at {left},{top} {width}x{height}"
      );
    }
    assert!(later.deferred().is_some(), "asking for parts did not make the picture");
  }

  #[test]
  fn a_chain_of_deferred_cpu_effects_gives_a_part_as_the_whole_does() {
    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    clear_gpu_provider();
    let base = ramp(8, 6);
    let mut whole = base.clone();
    ShiftLeft { options: None }.apply_to_image(&mut whole);
    ShiftLeft { options: None }.apply_to_image(&mut whole);
    let mut later = base.clone();
    ShiftLeft { options: None }.apply(deferred(&mut later));
    ShiftLeft { options: None }.apply(deferred(&mut later));
    // The second effect needs a pixel past the part, and the first needs one past that.
    assert_eq!(later.region(0, 3, 2, 2, 2), whole.region(0, 3, 2, 2, 2));
    assert_eq!(later.region(0, 6, 5, 4, 3), whole.region(0, 6, 5, 4, 3));
  }

  #[test]
  fn a_zoomed_out_part_comes_from_the_effect_for_that_level_or_from_the_whole_picture() {
    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    clear_gpu_provider();
    let base = ramp(8, 6);
    // An effect that knows its level is run on the level: the shift of one pixel is made on the halved picture.
    let mut known = base.clone();
    ShiftLeft { options: None }.apply(deferred(&mut known));
    let mut halved = ramp(8, 6);
    let halved_pixels = halved.region(1, 0, 0, 4, 3);
    let mut expected = Image::new_from_pixels(4, 3, halved_pixels, Channels::RGBA);
    ShiftLeft { options: None }.apply_to_image(&mut expected);
    assert_eq!(known.region(1, 0, 0, 4, 3), expected.to_rgba_vec());
    // One that does not is made whole, and its level is cut from that: the shift of one pixel, then halved.
    let mut unknown = base.clone();
    Unscaled(ShiftLeft { options: None }).apply(deferred(&mut unknown));
    let mut shifted = base.clone();
    ShiftLeft { options: None }.apply_to_image(&mut shifted);
    assert_eq!(unknown.region(1, 0, 0, 4, 3), shifted.region(1, 0, 0, 4, 3));
  }

  #[test]
  fn a_run_over_a_cpu_effect_starts_from_that_effect_and_keeps_the_shaders_after_it() {
    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    fake_gpu();
    let mut image = photo();
    ShiftLeft { options: None }.apply(deferred(&mut image));
    red_99().apply(deferred(&mut image));
    let (root, passes) = gpu_run(&image).expect("a run");
    assert!(root.deferred().is_some(), "the root is the CPU effect, which is made a part at a time");
    assert_eq!(passes.len(), 1, "only the shader is left to run over it");
    // Reading the whole thing runs the CPU effect, then the shader over it.
    assert_eq!(image.get_pixel(0, 0), Some((9, 8, 7, 255)));
    clear_gpu_provider();
  }
}
