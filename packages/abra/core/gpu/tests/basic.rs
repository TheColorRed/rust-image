use anyhow::Result;
use gpu::context::GpuContext;

#[test]
fn create_context_and_compile_shader() -> Result<()> {
  env_logger::init();
  let ctx = GpuContext::new_default_blocking()?;
  let _module = ctx.compile_wgsl("@compute @workgroup_size(1) fn main() { }", Some("test"));
  Ok(())
}
