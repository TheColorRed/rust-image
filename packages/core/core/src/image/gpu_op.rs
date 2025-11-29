/// GPU operation context for `process_image`.
///
/// This module provides a tiny thread-local context used to communicate
/// what operation (if any) a caller wants to run on the GPU during a
/// `process_image` invocation without changing `ApplyContext` signatures.
use std::cell::Cell;

/// Supported (initial) GPU operations. Add new variants as needed for
/// additional adjustments and filters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GpuOp {
  None,
  Brightness(f32),
  Contrast(f32),
}

impl Default for GpuOp {
  fn default() -> Self {
    GpuOp::None
  }
}

thread_local! {
  static CURRENT_GPU_SHADER: Cell<Option<String>> = Cell::new(None);
  static CURRENT_GPU_OP: Cell<GpuOp> = Cell::new(GpuOp::None);
}

/// Set the current GPU operation (used by adjustments/filters)
pub fn set_gpu_op(shader: impl Into<String>, op: GpuOp) {
  CURRENT_GPU_SHADER.with(|c| c.set(Some(shader.into())));
  CURRENT_GPU_OP.with(|c| c.set(op));
}

/// Clear the current GPU operation (set to None)
pub fn clear_gpu_op() {
  CURRENT_GPU_SHADER.with(|c| c.set(None));
  CURRENT_GPU_OP.with(|c| c.set(GpuOp::None));
}

/// Get the current GPU operation.
pub fn get_gpu_op() -> GpuOp {
  CURRENT_GPU_OP.with(|c| c.get())
}
/// Get the current GPU shader source, if any.
pub fn get_gpu_shader() -> Option<String> {
  CURRENT_GPU_SHADER.with(|c| c.take())
}
