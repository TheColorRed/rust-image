//! GPU-backed image utilities
//!
//! Helpers to upload and download textures to/from the GPU and construct
//! `abra_core::Image` objects for testing and integration.

use crate::context::GpuContext;
use anyhow::Result;

/// A GPU-backed texture.
pub struct GpuImage {
  /// The raw GPU texture handle.
  pub texture: wgpu::Texture,
  /// A default mip-level view of the texture.
  pub view: wgpu::TextureView,
  /// Width in pixels.
  pub width: u32,
  /// Height in pixels.
  pub height: u32,
  /// The texture format used for this image.
  pub format: wgpu::TextureFormat,
}

impl GpuImage {
  /// Upload an `abra_core::Image` into a GPU texture.
  pub fn from_image(ctx: &GpuContext, image: &abra_core::Image) -> Result<Self> {
    let (width, height) = image.dimensions::<u32>();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let size = wgpu::Extent3d {
      width,
      height,
      depth_or_array_layers: 1,
    };
    let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
      label: Some("gpu::upload_texture"),
      size,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format,
      usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
      view_formats: &[],
    });
    let rgba = image.rgba();
    let bytes_per_row = 4 * width as u32;
    ctx.queue.write_texture(
      wgpu::TexelCopyTextureInfo {
        texture: &texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      &rgba,
      wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(bytes_per_row),
        rows_per_image: Some(height),
      },
      size,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    Ok(Self {
      texture,
      view,
      width,
      height,
      format,
    })
  }

  /// Download the GPU image as a `abra_core::Image`. This function blocks using `pollster`.
  pub fn to_image_blocking(&self, ctx: &GpuContext) -> Result<abra_core::Image> {
    let unpadded_bytes_per_row = 4 * self.width as u32;
    let align: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // WebGPU required alignment for buffer rows
    let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
    let buffer_size = (padded_bytes_per_row as u64) * (self.height as u64);

    let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("gpu::readback_buffer"),
      size: buffer_size,
      usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("gpu::copy_texture_to_buffer"),
    });
    encoder.copy_texture_to_buffer(
      wgpu::TexelCopyTextureInfo {
        texture: &self.texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      wgpu::TexelCopyBufferInfo {
        buffer: &buffer,
        layout: wgpu::TexelCopyBufferLayout {
          offset: 0,
          bytes_per_row: Some(padded_bytes_per_row),
          rows_per_image: Some(self.height),
        },
      },
      wgpu::Extent3d {
        width: self.width,
        height: self.height,
        depth_or_array_layers: 1,
      },
    );

    ctx.queue.submit(Some(encoder.finish()));
    // wait until device is idle
    ctx.device.poll(wgpu::PollType::wait_indefinitely())?;

    let slice = buffer.slice(..);
    // map_async is callback based in wgpu 0.27
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    slice.map_async(wgpu::MapMode::Read, move |res| {
      let _ = tx.send(res);
    });
    // Ensure the device polls so the map async callback can run.
    ctx.device.poll(wgpu::PollType::wait_indefinitely())?;
    let res = rx.recv().map_err(|_| anyhow::anyhow!("map_async callback failed"))?;
    res?;
    let data = slice.get_mapped_range();

    // Copy rows into a compact vector of rgba pixels
    let mut pixels = vec![0u8; (self.width * self.height * 4) as usize];
    for y in 0..self.height as usize {
      let src_start = (y as u64 * padded_bytes_per_row as u64) as usize;
      let src_end = src_start + unpadded_bytes_per_row as usize;
      let dst_start = y * unpadded_bytes_per_row as usize;
      pixels[dst_start..dst_start + unpadded_bytes_per_row as usize].copy_from_slice(&data[src_start..src_end]);
    }

    drop(data);
    buffer.unmap();

    // Construct an Image from pixels
    let img = abra_core::Image::new_from_pixels(self.width, self.height, pixels, abra_core::Channels::RGBA);
    Ok(img)
  }
}
