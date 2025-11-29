//! Small runtime registry for optional GPU providers.
//!
//! `core` must stay independent of the `gpu` crate; this module provides a tiny
//! callback-based registry so an implementation in `packages/gpu` can register a
//! provider at runtime and `process_image` can call it without importing `gpu`.
use crate::image::apply_area::PreparedAreaMeta;
use once_cell::sync::Lazy;
use std::sync::{Arc, RwLock};

/// Minimal provider callbacks for GPU processing.
///
/// We keep this callback interface intentionally small and `String`-based for errors
/// so `core` never needs to know about GPU-specific types.
pub struct GpuCallback {
  pub should_process: Arc<dyn Fn(&PreparedAreaMeta) -> bool + Send + Sync>,
  pub process: Arc<dyn Fn(&PreparedAreaMeta, &[u8]) -> Result<Vec<u8>, String> + Send + Sync>,
}

static GPU_PROVIDER: Lazy<RwLock<Option<Arc<GpuCallback>>>> = Lazy::new(|| RwLock::new(None));

/// Register a GPU provider callback. Replaces any previously registered provider.
pub fn register_gpu_provider(provider: Arc<GpuCallback>) {
  let mut w = GPU_PROVIDER.write().unwrap();
  *w = Some(provider);
}

/// Clear the registered GPU provider (used in tests or to disable GPU at runtime).
pub fn clear_gpu_provider() {
  let mut w = GPU_PROVIDER.write().unwrap();
  *w = None;
}

/// Internal helper for `core` to obtain the registered provider if any.
pub fn get_gpu_provider() -> Option<Arc<GpuCallback>> {
  GPU_PROVIDER.read().unwrap().clone()
}
