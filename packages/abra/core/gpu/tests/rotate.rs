//! The GPU rotation gives the CPU's pixels, to within the rounding of 32-bit numbers on the GPU against the CPU's 64-bit position.

use abra_core::image::gpu::{GpuProcessor, Hardware, ready_gpu_provider};
use abra_core::transform::{TransformAlgorithm, TransformFit, rotate};
use abra_core::{Channels, Image};

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

/// Rotates the same picture on each hardware and returns how many values differ by more than 2, and the largest difference.
fn differences(p_algorithm: TransformAlgorithm, p_fit: TransformFit, p_degrees: f32) -> (usize, u8) {
  let source = picture(37, 23);
  let mut on_cpu = source.clone();
  rotate(p_degrees).with_fit(p_fit).with_algorithm(p_algorithm).with_hardware(Hardware::Cpu).apply(&mut on_cpu);
  let mut on_gpu = source.clone();
  rotate(p_degrees).with_fit(p_fit).with_algorithm(p_algorithm).with_hardware(Hardware::Gpu).apply(&mut on_gpu);
  assert_eq!(on_gpu.dimensions::<u32>(), on_cpu.dimensions::<u32>(), "{p_algorithm:?} {p_fit:?} {p_degrees}");
  let gaps: Vec<u8> = on_cpu.rgba().iter().zip(on_gpu.rgba()).map(|(cpu, gpu)| cpu.abs_diff(*gpu)).collect();
  (gaps.iter().filter(|gap| **gap > 2).count(), gaps.into_iter().max().unwrap_or(0))
}

#[test]
fn the_gpu_rotation_matches_the_cpu_for_every_algorithm() {
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
    for fit in [TransformFit::Expand, TransformFit::Crop, TransformFit::Fill] {
      for degrees in [90.0f32, 7.0, -20.0, 33.5, 145.0] {
        let (count, worst) = differences(algorithm, fit, degrees);
        println!("{algorithm:?} {fit:?} {degrees}: {count} values over 2, worst {worst}");
        // The nearest pixel and the edge-directed choices jump when a 32-bit spot lands on the other side of a boundary, which
        // is rare and gives a whole pixel's difference; the smooth kernels move by a level or two at most.
        let jumpy = matches!(
          algorithm,
          TransformAlgorithm::NearestNeighbor | TransformAlgorithm::EdgeDirectNEDI | TransformAlgorithm::EdgeDirectEDI
        );
        if jumpy {
          assert!(count <= 40, "{algorithm:?} {fit:?} {degrees}: {count} values differ");
        } else {
          assert!(worst <= 2, "{algorithm:?} {fit:?} {degrees} differs by {worst}");
        }
      }
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
    let step = rotate(30.0).with_algorithm(algorithm);
    assert_eq!(step.passes(17, 11).len(), 1, "{algorithm:?}");
  }
}
