use ctor::ctor;
use gpu::context::GpuContext;
use std::sync::Arc;
// Use ctor::ctor attribute directly instead of importing `ctor` helper into scope.

/// Initialize and register a default GPU context provider with core's registry.
/// Returns `Ok(true)` if registration occurred successfully.
pub fn init_gpu_blocking() -> anyhow::Result<()> {
  let ctx = Arc::new(GpuContext::new_default_blocking()?);
  gpu::register_gpu_context(ctx);
  Ok(())
}

/// Run automatic initialization at library load time when this crate is linked.
#[ctor]
fn init_on_load() {
  // Spawn a background thread for initializing the GPU provider so crate load
  // doesn't block if adapter or driver enumeration hangs.
  std::thread::spawn(|| {
    // Best-effort registration; ignore errors.
    match init_gpu_blocking() {
      Ok(_) => println!("gpu_integration: GPU provider registered"),
      Err(e) => println!("gpu_integration: GPU provider init failed: {:?}", e),
    }
  });
}
