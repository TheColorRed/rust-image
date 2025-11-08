use pollster::FutureExt;
use wgpu::{
  Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, BufferDescriptor, CommandEncoderDescriptor, ComputePipeline,
  ComputePipelineDescriptor, Device, Extent3d, ImageCopyBuffer, ImageCopyTexture, ImageDataLayout, Instance, InstanceDescriptor, Maintain,
  MapMode, Origin3d, PowerPreference, Queue, RequestAdapterOptionsBase, ShaderModuleDescriptor, ShaderSource, Texture, TextureAspect,
  TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
};

use crate::image::{Image, ShaderBinding};
use std::fs;

/// Prepares the GPU for rendering.
pub fn prepare_gpu() -> (Device, Queue) {
  let instance = Instance::new(InstanceDescriptor {
    backends: Backends::all(),
    ..Default::default()
  });
  let adapter = instance
    .request_adapter(&RequestAdapterOptionsBase {
      power_preference: PowerPreference::HighPerformance,
      force_fallback_adapter: false,
      compatible_surface: None,
    })
    .block_on()
    .ok_or("Failed to find an appropriate adapter with a queue that supports compute")
    .unwrap();
  let (device, queue) = adapter.request_device(&Default::default(), None).block_on().unwrap();
  (device, queue)
}

/// Loads an image into the GPU.
pub fn prepare_image(image: &Image, device: &Device, queue: &Queue) -> (Texture, Texture) {
  let (width, height) = image.dimensions();
  let texture_size = Extent3d {
    width,
    height,
    depth_or_array_layers: 1,
  };

  let input_texture = device.create_texture(&TextureDescriptor {
    size: texture_size,
    mip_level_count: 1,
    sample_count: 1,
    view_formats: &[],
    dimension: TextureDimension::D2,
    format: TextureFormat::Rgba8Unorm,
    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
    label: Some("input_texture"),
  });

  let output_texture = device.create_texture(&TextureDescriptor {
    size: texture_size,
    mip_level_count: 1,
    sample_count: 1,
    view_formats: &[],
    dimension: TextureDimension::D2,
    format: TextureFormat::Rgba8Unorm,
    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC | TextureUsages::STORAGE_BINDING,
    label: Some("output_texture"),
  });

  queue.write_texture(
    ImageCopyTexture {
      texture: &input_texture,
      mip_level: 0,
      origin: Default::default(),
      aspect: Default::default(),
    },
    image.rgba().as_slice(),
    wgpu::ImageDataLayout {
      offset: 0,
      bytes_per_row: Some(4 * width),
      rows_per_image: None,
    },
    texture_size,
  );

  (input_texture, output_texture)
}

/// Loads a shader from a file path.
pub fn load_shader_from_path(
  path: &str,
  device: &Device,
  input_texture: &Texture,
  output_texture: &Texture,
  bindings: Option<Vec<ShaderBinding>>,
) -> (ComputePipeline, BindGroup) {
  // read shader from file using path
  let shader_code = fs::read_to_string(path).expect("Unable to read shader file.");
  load_shader_from_string(&shader_code, device, input_texture, output_texture, bindings)
}

/// Loads a shader from a string.
pub fn load_shader_from_string(
  str: &str,
  device: &Device,
  input_texture: &Texture,
  output_texture: &Texture,
  bindings: Option<Vec<ShaderBinding>>, // BindGroupEntry
) -> (ComputePipeline, BindGroup) {
  let shader = device.create_shader_module(ShaderModuleDescriptor {
    label: Some("shader"),
    source: ShaderSource::Wgsl(str.into()),
  });

  let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
    label: Some("pipeline"),
    layout: None,
    module: &shader,
    entry_point: Some("main"),
    cache: None,
    compilation_options: Default::default(),
  });

  let input_texture_view = input_texture.create_view(&TextureViewDescriptor::default());
  let output_texture_view = output_texture.create_view(&TextureViewDescriptor::default());

  let mut bind_entries = vec![
    BindGroupEntry {
      binding: 0,
      resource: BindingResource::TextureView(&input_texture_view),
    },
    BindGroupEntry {
      binding: 1,
      resource: BindingResource::TextureView(&output_texture_view),
    },
  ];

  // if let Some(bindings) = bindings {
  //   for binding in bindings {
  //     let buffer = device.create_buffer(&BufferDescriptor {
  //       label: Some("buffer"),
  //       size: 4,
  //       usage: wgpu::BufferUsages::all(),
  //       mapped_at_creation: false,
  //     });

  //     bind_entries.push(BindGroupEntry {
  //       binding: bind_entries.len() as u32,
  //       resource: BindingResource::Buffer(buffer.slice(..)),
  //     });
  //   }
  // }

  let texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
    label: Some("texture_bind_group"),
    layout: &pipeline.get_bind_group_layout(0),
    entries: bind_entries.as_slice(),
  });

  (pipeline, texture_bind_group)
}

pub fn apply_shader(
  image: &mut Image,
  device: &Device,
  queue: &Queue,
  pipeline: &ComputePipeline,
  texture_bind_group: &BindGroup,
  output_texture: &Texture,
) {
  let (width, height) = image.dimensions();
  let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

  let (dispatch_width, dispatch_height) = compute_work_group_count((width, height), (16, 16));
  let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
    label: Some("pass"),
    timestamp_writes: None,
  });
  compute_pass.set_pipeline(&pipeline);
  compute_pass.set_bind_group(0, texture_bind_group, &[]);
  compute_pass.dispatch_workgroups(dispatch_width, dispatch_height, 1);
  drop(compute_pass); // End the compute pass

  let texture_size = Extent3d {
    width,
    height,
    depth_or_array_layers: 1,
  };

  let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("output_buffer"),
    size: (4 * width * height) as u64,
    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    mapped_at_creation: false,
  });

  let padded_bytes_per_row = padded_bytes_per_row(width);
  let unpadded_bytes_per_row = width as usize * 4;

  encoder.copy_texture_to_buffer(
    ImageCopyTexture {
      aspect: TextureAspect::All,
      texture: &output_texture,
      mip_level: 0,
      origin: Origin3d::ZERO,
    },
    ImageCopyBuffer {
      buffer: &output_buffer,
      layout: ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(padded_bytes_per_row as u32),
        rows_per_image: Some(height),
      },
    },
    texture_size,
  );

  // Submit the command encoder to the queue
  queue.submit(Some(encoder.finish()));
  let buffer_slice = output_buffer.slice(..);
  buffer_slice.map_async(MapMode::Read, |_| {});
  device.poll(Maintain::Wait);

  let padded_data = buffer_slice.get_mapped_range();
  let mut pixels: Vec<u8> = vec![0; unpadded_bytes_per_row * height as usize];

  for (padded, pixels) in padded_data
    .chunks_exact(padded_bytes_per_row)
    .zip(pixels.chunks_exact_mut(unpadded_bytes_per_row))
  {
    pixels.copy_from_slice(&padded[..unpadded_bytes_per_row]);
  }

  image.set_rgba(pixels);
}

/// Computes the number of work groups needed to process the image.
fn compute_work_group_count((width, height): (u32, u32), (workgroup_width, workgroup_height): (u32, u32)) -> (u32, u32) {
  let x = (width + workgroup_width - 1) / workgroup_width;
  let y = (height + workgroup_height - 1) / workgroup_height;

  (x, y)
}

fn padded_bytes_per_row(width: u32) -> usize {
  let bytes_per_row = width as usize * 4;
  let padding = (256 - bytes_per_row % 256) % 256;
  bytes_per_row + padding
}
