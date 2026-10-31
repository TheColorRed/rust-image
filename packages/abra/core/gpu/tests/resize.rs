//! The GPU resize gives the CPU's pixels, to within the rounding of 32-bit numbers on the GPU against the CPU's 64-bit position.

use abra_core::image::gpu::{GpuProcessor, Hardware, ready_gpu_provider};
use abra_core::transform::{TransformAlgorithm, resize};
use abra_core::{Channels, Image, ResizeTarget, Size};

/// A picture with colors that change in both directions, and a band that is partly see-through, so premultiplying matters.
fn picture(p_width: u32, p_height: u32) -> Image {
  let mut pixels = Vec::new();
  for y in 0..p_height {
    for x in 0..p_width {
      let alpha = if (p_height / 3..2 * p_height / 3).contains(&y) { 90 + (x * 7 % 120) as u8 } else { 255 };
      pixels.extend_from_slice(&[(x * 13 + y * 3) as u8, (y * 29) as u8, (x * y * 5) as u8, alpha]);
    }
  }
  Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA)
}

/// Resizes the same picture on each hardware and returns how many values differ by more than 2, and the largest difference.
fn differences(p_algorithm: TransformAlgorithm, p_from: (u32, u32), p_to: (u32, u32)) -> (usize, u8) {
  let source = picture(p_from.0, p_from.1);
  let size = ResizeTarget::Exact(Size::new(p_to.0, p_to.1));
  let mut on_cpu = source.clone();
  resize(size).with_algorithm(p_algorithm).with_hardware(Hardware::Cpu).apply(&mut on_cpu);
  let mut on_gpu = source.clone();
  resize(size).with_algorithm(p_algorithm).with_hardware(Hardware::Gpu).apply(&mut on_gpu);
  assert_eq!(on_cpu.dimensions::<u32>(), p_to);
  assert_eq!(on_gpu.dimensions::<u32>(), p_to, "{p_algorithm:?} {p_from:?} to {p_to:?}");
  let gaps: Vec<u8> = on_cpu.rgba().iter().zip(on_gpu.rgba()).map(|(cpu, gpu)| cpu.abs_diff(*gpu)).collect();
  (gaps.iter().filter(|gap| **gap > 2).count(), gaps.into_iter().max().unwrap_or(0))
}

const SIZES: [((u32, u32), (u32, u32)); 4] = [
  // Smaller, much smaller, larger, and a different shape that is a little larger in one direction.
  ((17, 11), (8, 5)),
  ((40, 30), (7, 4)),
  ((9, 6), (31, 20)),
  ((31, 19), (31, 20)),
];

#[test]
fn the_gpu_resize_matches_the_cpu_for_every_algorithm() {
  gpu::register();
  assert!(ready_gpu_provider(Hardware::Gpu).is_some(), "these tests need a GPU");
  for algorithm in [
    TransformAlgorithm::NearestNeighbor,
    TransformAlgorithm::Bilinear,
    TransformAlgorithm::Bicubic,
    TransformAlgorithm::Lanczos,
    TransformAlgorithm::EdgeDirectNEDI,
    TransformAlgorithm::EdgeDirectEDI,
  ] {
    for (from, to) in SIZES {
      let (_, worst) = differences(algorithm, from, to);
      println!("{algorithm:?} {from:?} to {to:?}: worst difference {worst}");
      assert!(worst <= 2, "{algorithm:?} {from:?} to {to:?} differs by {worst}");
    }
  }
}

#[test]
fn every_algorithm_has_a_shader() {
  for algorithm in [
    TransformAlgorithm::NearestNeighbor,
    TransformAlgorithm::Bilinear,
    TransformAlgorithm::Bicubic,
    TransformAlgorithm::Lanczos,
    TransformAlgorithm::EdgeDirectNEDI,
    TransformAlgorithm::EdgeDirectEDI,
    TransformAlgorithm::Auto,
  ] {
    let step = resize(ResizeTarget::Exact(Size::new(6, 4))).with_algorithm(algorithm);
    assert_eq!(step.passes(17, 11).len(), 1, "{algorithm:?}");
  }
}

#[test]
fn a_resize_that_changes_nothing_has_no_passes() {
  assert!(resize(ResizeTarget::Exact(Size::new(17, 11))).passes(17, 11).is_empty());
}
