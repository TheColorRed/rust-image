use anyhow::Result;
use gpu::context::GpuContext;
use gpu::image::GpuImage;

#[test]
fn create_context_and_compile_shader() -> Result<()> {
  env_logger::init();
  let ctx = GpuContext::new_default_blocking()?;
  let _module = ctx.compile_wgsl("@compute @workgroup_size(1) fn main() { }", Some("test"));
  Ok(())
}

#[test]
fn upload_download_roundtrip() -> Result<()> {
  let ctx = GpuContext::new_default_blocking()?;
  let mut image = abra_core::Image::new(2u32, 2u32);
  image.set_rgba(&[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255]);
  let gpu_image = GpuImage::from_image(&ctx, &image)?;
  let downloaded = gpu_image.to_image_blocking(&ctx)?;
  assert_eq!(image.rgba(), downloaded.rgba());
  Ok(())
}
