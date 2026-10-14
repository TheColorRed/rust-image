//! A photo of any size, drawn a window at a time.
//!
//! [`TiledRenderer`] holds a photo as a pyramid of tiles: the photo cut into squares, and the same photo halved again and
//! again, each cut into squares of its own. Only the tiles a window needs are on the GPU, so the photo can be bigger than
//! the GPU's largest texture and a zoomed view costs the screen and not the photo. A window (the part of the photo to show
//! and the size to show it at) is drawn from the level whose pixels are closest to the screen's, the effects run over that
//! window and nothing else, and the result is a texture the size of the screen.

use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};

use crate::context::{FORMAT, GpuContext};
use crate::{GpuPass, LiveRenderer};

/// Paints what is past the photo, after the effects have run, so they cannot tint it. The uniform is the color.
const OUTSIDE_SHADER: &str = r#"
struct Window {
  origin: vec2<f32>,
  scale: f32,
  scale_y: f32,
  full: vec2<f32>,
  pad2: vec2<f32>,
}
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> outside: vec4<f32>;
@group(0) @binding(5) var<uniform> window: Window;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let at = vec2<i32>(gid.xy);
  let photo = window.origin + (vec2<f32>(gid.xy) + 0.5) * vec2<f32>(window.scale, window.scale_y);
  let inside = all(photo >= vec2<f32>(0.0)) && all(photo < window.full);
  textureStore(output_tex, at, select(outside, textureLoad(input_tex, at, 0), inside));
}
"#;

/// The size of one quad's uniform, and the distance between two (a dynamic offset must be a multiple of 256).
const QUAD_STRIDE: u64 = 256;

/// How a [`TiledRenderer`] cuts up, keeps and draws a photo. [`Default`] is a reasonable start for a phone or a desktop;
/// every value is the caller's to change, since only the app knows its screen, its memory and what it is showing.
///
/// ```ignore
/// let options = TileOptions::default().with_tile_size(1024).with_budget_bytes(64 << 20).with_filter(wgpu::FilterMode::Nearest);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct TileOptions {
  /// How many pixels a tile is across. Bigger tiles mean fewer draws and less bookkeeping, but more upload and memory for a
  /// tile that is only partly in view. Default 512.
  pub tile_size: u32,
  /// What the tiles on the GPU may add up to, in bytes, before the ones not drawn lately are let go. Tiles in the window
  /// being drawn are never let go, so a window that needs more than this still draws. Default 256 MiB.
  pub budget_bytes: u64,
  /// Most tiles one window draws; a window that needs more leaves the rest out. Default 256.
  pub max_drawn: usize,
  /// Most padding, in pixels, a window gets all the way round. The renderer pads a window by what its effects reach (a
  /// blur's radius at this zoom), and a wide blur on a deep zoom would otherwise ask for more than the screen. Past it the
  /// pixels at the very edge of the picture see their own edge where their neighbours should be. Default 256.
  pub max_pad: u32,
  /// How much sharper (negative) or softer (positive) than the screen the level of detail is, in halvings. At 0 a level is
  /// used while its pixels are no smaller than the screen's; at -1 the next, sharper level is used, which costs more
  /// upload and looks crisper when the picture is shrunk. Default 0.
  pub level_bias: f32,
  /// How the tiles are sampled when a view is magnified or shrunk: smooth (`Linear`) or showing each pixel (`Nearest`).
  /// Default `Linear`.
  pub filter: wgpu::FilterMode,
  /// The color, straight RGBA from 0 to 1, of the part of a window that is past the photo. Default clear, which an app can
  /// tell from the photo and draw as it likes.
  pub outside: [f32; 4],
  /// The most time one frame spends putting tiles on the GPU. A tile that is wanted and has to wait for a later frame is
  /// drawn from the next blurrier level that is on the GPU meanwhile, so a pinch or a pan stays smooth and sharpens a moment
  /// later; [`TiledRenderer::wants_more`] tells the caller to draw again. At least one tile is put on in every frame, and a
  /// tile with no blurrier stand-in (the first view of a photo) is always put on at once. `Duration::MAX` puts on every tile
  /// the window needs in the frame that needs it. Default 4 ms.
  pub load_budget: Duration,
  /// How many tiles past the edge of the window to put on the GPU ahead of time, with the time left after the window's own
  /// tiles, along with the sharper level's tiles under the window. They are ready when the view moves or zooms in. 0 puts
  /// on nothing that is not drawn. Default 0, since only the app knows how much memory it can spend on what may not be seen.
  pub prefetch: u32,
}

impl Default for TileOptions {
  fn default() -> Self {
    Self {
      tile_size: 512,
      budget_bytes: 256 * 1024 * 1024,
      max_drawn: 256,
      max_pad: 256,
      level_bias: 0.0,
      filter: wgpu::FilterMode::Linear,
      outside: [0.0; 4],
      load_budget: Duration::from_millis(4),
      prefetch: 0,
    }
  }
}

impl TileOptions {
  /// Sets [`tile_size`](Self::tile_size).
  pub fn with_tile_size(mut self, p_pixels: u32) -> Self {
    self.tile_size = p_pixels;
    self
  }

  /// Sets [`budget_bytes`](Self::budget_bytes).
  pub fn with_budget_bytes(mut self, p_bytes: u64) -> Self {
    self.budget_bytes = p_bytes;
    self
  }

  /// Sets [`max_drawn`](Self::max_drawn).
  pub fn with_max_drawn(mut self, p_tiles: usize) -> Self {
    self.max_drawn = p_tiles;
    self
  }

  /// Sets [`max_pad`](Self::max_pad).
  pub fn with_max_pad(mut self, p_pixels: u32) -> Self {
    self.max_pad = p_pixels;
    self
  }

  /// Sets [`level_bias`](Self::level_bias).
  pub fn with_level_bias(mut self, p_halvings: f32) -> Self {
    self.level_bias = p_halvings;
    self
  }

  /// Sets [`filter`](Self::filter).
  pub fn with_filter(mut self, p_filter: wgpu::FilterMode) -> Self {
    self.filter = p_filter;
    self
  }

  /// Sets [`outside`](Self::outside).
  pub fn with_outside(mut self, p_rgba: [f32; 4]) -> Self {
    self.outside = p_rgba;
    self
  }

  /// Sets [`load_budget`](Self::load_budget).
  pub fn with_load_budget(mut self, p_time: Duration) -> Self {
    self.load_budget = p_time;
    self
  }

  /// Sets [`prefetch`](Self::prefetch).
  pub fn with_prefetch(mut self, p_tiles: u32) -> Self {
    self.prefetch = p_tiles;
    self
  }
}

const TILE_SHADER: &str = r#"
struct Quad {
  // The corners in clip space: left, top, right, bottom.
  rect: vec4<f32>,
  // The same corners in the tile's texture: left, top, right, bottom.
  uv: vec4<f32>,
};

@group(0) @binding(0) var tile: texture_2d<f32>;
@group(0) @binding(1) var tile_sampler: sampler;
@group(0) @binding(2) var<uniform> quad: Quad;

struct VsOut {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) index: u32) -> VsOut {
  var corners = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0),
  );
  let corner = corners[index];
  var out: VsOut;
  out.position = vec4<f32>(mix(quad.rect.x, quad.rect.z, corner.x), mix(quad.rect.y, quad.rect.w, corner.y), 0.0, 1.0);
  out.uv = vec2<f32>(mix(quad.uv.x, quad.uv.z, corner.x), mix(quad.uv.y, quad.uv.w, corner.y));
  return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
  return textureSample(tile, tile_sampler, in.uv);
}
"#;

/// The part of a photo to draw, and the size to draw it at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
  /// The left edge, in the photo's own pixels.
  pub x: f32,
  /// The top edge, in the photo's own pixels.
  pub y: f32,
  /// How much of the photo's width, in its own pixels.
  pub width: f32,
  /// How much of the photo's height, in its own pixels.
  pub height: f32,
  /// The width of the picture drawn, in pixels.
  pub out_width: u32,
  /// The height of the picture drawn, in pixels.
  pub out_height: u32,
  /// Extra pixels drawn all the way round, so an effect that looks at its neighbours has them at the edge of the picture.
  /// They are not part of the view; see [`TiledRenderer::render`].
  pub pad: u32,
}

/// What gives a [`TiledRenderer`] the pixels of its tiles: a function that returns a rectangle of the photo at a level.
///
/// It is called as `regions(level, left, top, width, height)`. Level 0 is the photo and each level above it is the one before
/// halved, rounding a side that is odd up. The rectangle is in the pixels of that level and may reach past its edges, where
/// a pixel is the pixel at the edge. It returns `width * height` RGBA pixels. It is called on the thread that draws, a tile
/// at a time and only for the tiles in view, so a photo too big to hold, or one that is made as it is asked for, costs what
/// is on the screen.
pub type Regions = Arc<dyn Fn(u32, i32, i32, u32, u32) -> Vec<u8> + Send + Sync>;

/// One level of the pyramid: the photo at some size, as pixels on the CPU.
struct Level {
  width: u32,
  height: u32,
  pixels: Arc<[u8]>,
}

/// Where the pixels of a photo's levels come from.
enum Source {
  /// The photo's own pixels, from which the levels above are made the first time a window needs them.
  Pixels(Mutex<Vec<Level>>),
  /// A function that makes any part of any level.
  Regions(Regions),
}

/// The photo halved again and again, down to one pixel. With pixels, a level is made the first time a window needs it, so a
/// photo that is only ever drawn at its own size costs nothing for the rest.
struct Pyramid {
  width: u32,
  height: u32,
  /// How many levels there are in all, made or not.
  count: usize,
  source: Source,
}

impl Pyramid {
  fn counted(p_width: u32, p_height: u32) -> usize {
    let (mut width, mut height, mut count) = (p_width, p_height, 1);
    while width > 1 || height > 1 {
      (width, height) = (width.div_ceil(2), height.div_ceil(2));
      count += 1;
    }
    count
  }

  fn new(p_width: u32, p_height: u32, p_rgba: Arc<[u8]>) -> Pyramid {
    Pyramid {
      width: p_width,
      height: p_height,
      count: Self::counted(p_width, p_height),
      source: Source::Pixels(Mutex::new(vec![Level { width: p_width, height: p_height, pixels: p_rgba }])),
    }
  }

  fn from_regions(p_width: u32, p_height: u32, p_regions: Regions) -> Pyramid {
    Pyramid { width: p_width, height: p_height, count: Self::counted(p_width, p_height), source: Source::Regions(p_regions) }
  }

  /// The size of the photo at `p_level`.
  fn dims(&self, p_level: usize) -> (u32, u32) {
    let (mut width, mut height) = (self.width, self.height);
    for _ in 0..p_level {
      (width, height) = (width.div_ceil(2), height.div_ceil(2));
    }
    (width, height)
  }

  /// The rectangle `p_width` x `p_height` at `(p_left, p_top)` of `p_level`, with the pixel at the edge for any past it.
  fn region(&self, p_level: usize, p_left: i32, p_top: i32, p_width: u32, p_height: u32) -> Vec<u8> {
    let levels = match &self.source {
      Source::Regions(regions) => return regions(p_level as u32, p_left, p_top, p_width, p_height),
      Source::Pixels(levels) => levels,
    };
    let mut levels = levels.lock().unwrap();
    // Makes the levels up to the one asked for, each from the one above it.
    while levels.len() <= p_level.min(self.count - 1) {
      let last = levels.last().expect("level 0 is always there");
      let (width, height) = (last.width.div_ceil(2), last.height.div_ceil(2));
      let mut pixels = vec![0u8; width as usize * height as usize * 4];
      for y in 0..height {
        for x in 0..width {
          // The mean of the up to four pixels it covers, in each channel, rounded.
          let mut sum = [0u32; 4];
          let mut count = 0;
          for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let (source_x, source_y) = (x * 2 + dx, y * 2 + dy);
            if source_x < last.width && source_y < last.height {
              let at = (source_y as usize * last.width as usize + source_x as usize) * 4;
              for channel in 0..4 {
                sum[channel] += last.pixels[at + channel] as u32;
              }
              count += 1;
            }
          }
          let at = (y as usize * width as usize + x as usize) * 4;
          for channel in 0..4 {
            pixels[at + channel] = ((sum[channel] + count / 2) / count) as u8;
          }
        }
      }
      levels.push(Level { width, height, pixels: pixels.into() });
    }
    let level = &levels[p_level.min(levels.len() - 1)];
    let mut out = vec![0u8; p_width as usize * p_height as usize * 4];
    // The columns of the rectangle that are inside the level: from the first with a source column of 0 or more, up to the first
    // that is past the last.
    let inside_from = (-(p_left as i64)).clamp(0, p_width as i64) as usize;
    let inside_to = (level.width as i64 - p_left as i64).clamp(inside_from as i64, p_width as i64) as usize;
    for row in 0..p_height as usize {
      let source_y = (p_top as i64 + row as i64).clamp(0, level.height as i64 - 1) as usize;
      let line = &level.pixels[source_y * level.width as usize * 4..(source_y + 1) * level.width as usize * 4];
      let to = &mut out[row * p_width as usize * 4..(row + 1) * p_width as usize * 4];
      for column in 0..inside_from {
        to[column * 4..column * 4 + 4].copy_from_slice(&line[..4]);
      }
      let source_from = (p_left as i64 + inside_from as i64) as usize * 4;
      to[inside_from * 4..inside_to * 4].copy_from_slice(&line[source_from..source_from + (inside_to - inside_from) * 4]);
      for column in inside_to..p_width as usize {
        to[column * 4..column * 4 + 4].copy_from_slice(&line[line.len() - 4..]);
      }
    }
    out
  }
}

/// A tile on the GPU: a square of a level, with a pixel of its neighbours all the way round so a smooth lookup near its edge
/// sees them and no seam shows.
struct Tile {
  _texture: wgpu::Texture,
  bind_group: wgpu::BindGroup,
  last_drawn: u64,
}

/// The tiles of a photo that are on the GPU, and how they are drawn into a window.
struct TileCache {
  ctx: GpuContext,
  pyramid: Pyramid,
  options: TileOptions,
  tiles: HashMap<(usize, u32, u32), Tile>,
  frame: u64,
  /// Whether the last window left tiles for a later frame: a wanted tile drawn from a blurrier one, or one still to fetch.
  wants_more: bool,
  pipeline: wgpu::RenderPipeline,
  layout: wgpu::BindGroupLayout,
  sampler: wgpu::Sampler,
  quads: wgpu::Buffer,
}

impl TileCache {
  fn new(p_ctx: GpuContext, p_pyramid: Pyramid, p_options: TileOptions) -> TileCache {
    let device = &p_ctx.device;
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: Some("tiles::layout"),
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
          },
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 2,
          visibility: wgpu::ShaderStages::VERTEX,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: true,
            min_binding_size: NonZeroU64::new(32),
          },
          count: None,
        },
      ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("tiles::pipeline_layout"),
      bind_group_layouts: &[Some(&layout)],
      immediate_size: 0,
    });
    let shader = p_ctx.compile_wgsl(TILE_SHADER, Some("tiles::shader"));
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("tiles::pipeline"),
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs"),
        compilation_options: Default::default(),
        targets: &[Some(FORMAT.into())],
      }),
      primitive: wgpu::PrimitiveState::default(),
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview_mask: None,
      cache: None,
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      mag_filter: p_options.filter,
      min_filter: p_options.filter,
      ..Default::default()
    });
    let quads = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("tiles::quads"),
      size: QUAD_STRIDE * p_options.max_drawn as u64,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    TileCache {
      ctx: p_ctx,
      pyramid: p_pyramid,
      options: p_options,
      tiles: HashMap::new(),
      frame: 0,
      wants_more: false,
      pipeline,
      layout,
      sampler,
      quads,
    }
  }

  /// Puts the tile on the GPU, if it is not there already.
  fn load(&mut self, p_level: usize, p_x: u32, p_y: u32) {
    if self.tiles.contains_key(&(p_level, p_x, p_y)) {
      return;
    }
    let side = self.options.tile_size + 2;
    // The tile's own square of the level and a pixel of its neighbours all the way round.
    let pixels = self.pyramid.region(
      p_level,
      (p_x as i64 * self.options.tile_size as i64 - 1) as i32,
      (p_y as i64 * self.options.tile_size as i64 - 1) as i32,
      side,
      side,
    );
    let texture = self.ctx.device.create_texture(&wgpu::TextureDescriptor {
      label: Some("tiles::tile"),
      size: wgpu::Extent3d { width: side, height: side, depth_or_array_layers: 1 },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: FORMAT,
      usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
      view_formats: &[],
    });
    self.ctx.queue.write_texture(
      texture.as_image_copy(),
      &pixels,
      wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * side), rows_per_image: Some(side) },
      wgpu::Extent3d { width: side, height: side, depth_or_array_layers: 1 },
    );
    let view = texture.create_view(&Default::default());
    let bind_group = self.ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("tiles::bind_group"),
      layout: &self.layout,
      entries: &[
        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
        wgpu::BindGroupEntry {
          binding: 2,
          resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: &self.quads,
            offset: 0,
            size: NonZeroU64::new(32),
          }),
        },
      ],
    });
    self.tiles.insert((p_level, p_x, p_y), Tile { _texture: texture, bind_group, last_drawn: self.frame });
  }

  /// Draws the part of the photo `p_window` asks for into `p_target`, which is `p_size` pixels. Nothing is drawn where the
  /// window is past the photo, so it stays clear.
  fn draw(&mut self, p_window: &Window, p_target: &wgpu::TextureView, p_size: (u32, u32)) {
    let started = Instant::now();
    self.frame += 1;
    self.wants_more = false;
    let pad = p_window.pad as f32;
    // How many photo pixels one pixel of the picture is across and down. They differ when the window is stretched.
    let scale_x = p_window.width / p_window.out_width as f32;
    let scale_y = p_window.height / p_window.out_height as f32;
    // The photo's pixel at the top left of the picture drawn, padding included.
    let origin = (p_window.x - pad * scale_x, p_window.y - pad * scale_y);
    // The level whose pixels are the nearest to a screen pixel without being smaller, so a view is never sharper than it
    // needs to be and a smooth lookup never skips over pixels.
    let level = ((scale_x.min(scale_y).max(1.0).log2() + self.options.level_bias).floor().max(0.0) as usize).min(self.pyramid.count - 1);
    let step = (1u64 << level) as f32;
    let (level_width, level_height) = self.pyramid.dims(level);
    let tile = self.options.tile_size;
    let max_drawn = self.options.max_drawn;

    let reach = |from: f32, extent: u32, per_pixel: f32, limit: u32| -> std::ops::Range<u32> {
      let first = (from / step).floor().max(0.0) as u32 / tile;
      let last = (((from + extent as f32 * per_pixel) / step).ceil().max(0.0) as u32).min(limit);
      first..last.div_ceil(tile).max(first)
    };
    let columns = reach(origin.0, p_size.0, scale_x, level_width);
    let rows = reach(origin.1, p_size.1, scale_y, level_height);

    // The tiles the window needs, the middle ones first, so a frame that cannot put them all on the GPU does the middle.
    let middle = ((columns.start + columns.end) as f32 / 2.0, (rows.start + rows.end) as f32 / 2.0);
    let from_middle = |tile_x: u32, tile_y: u32| (tile_x as f32 + 0.5 - middle.0).powi(2) + (tile_y as f32 + 0.5 - middle.1).powi(2);
    let mut wanted: Vec<(u32, u32)> = rows.clone().flat_map(|tile_y| columns.clone().map(move |tile_x| (tile_x, tile_y))).collect();
    wanted.sort_by(|first, second| from_middle(first.0, first.1).total_cmp(&from_middle(second.0, second.1)));
    wanted.truncate(max_drawn);

    // What is drawn is the wanted tiles that are on the GPU, over a blurrier tile for each one that has to wait.
    let mut drawn = Vec::new();
    let mut stand_ins: Vec<(usize, u32, u32)> = Vec::new();
    let mut put_on = 0;
    for (tile_x, tile_y) in wanted {
      let key = (level, tile_x, tile_y);
      if !self.tiles.contains_key(&key) {
        // One tile is always put on, so a view that needs many still gets there. A tile with nothing blurrier to stand in
        // for it (the first view) is put on whatever the time.
        let stand_in = if put_on > 0 && started.elapsed() >= self.options.load_budget { self.stand_in(key) } else { None };
        if let Some(stand_in) = stand_in {
          self.wants_more = true;
          if !stand_ins.contains(&stand_in) {
            stand_ins.push(stand_in);
          }
          continue;
        }
        self.load(level, tile_x, tile_y);
        put_on += 1;
      }
      drawn.push(key);
    }

    // With the time left, the tiles around the window and the sharper level under it, for the view's next move.
    if !self.wants_more && self.options.prefetch > 0 {
      let (across, down) = (level_width.div_ceil(tile), level_height.div_ceil(tile));
      let around = self.options.prefetch;
      let mut ahead: Vec<(usize, u32, u32)> = Vec::new();
      for tile_y in rows.start.saturating_sub(around)..(rows.end + around).min(down) {
        for tile_x in columns.start.saturating_sub(around)..(columns.end + around).min(across) {
          ahead.push((level, tile_x, tile_y));
        }
      }
      if level > 0 {
        let sharper = self.pyramid.dims(level - 1);
        let (sharper_across, sharper_down) = (sharper.0.div_ceil(tile), sharper.1.div_ceil(tile));
        for tile_y in rows.start * 2..(rows.end * 2).min(sharper_down) {
          for tile_x in columns.start * 2..(columns.end * 2).min(sharper_across) {
            ahead.push((level - 1, tile_x, tile_y));
          }
        }
      }
      for (ahead_level, tile_x, tile_y) in ahead {
        if self.tiles.contains_key(&(ahead_level, tile_x, tile_y)) {
          continue;
        }
        if started.elapsed() >= self.options.load_budget {
          self.wants_more = true;
          break;
        }
        self.load(ahead_level, tile_x, tile_y);
      }
    }

    // The blurrier ones go first, and the sharper tiles over them.
    stand_ins.sort_by(|first, second| second.0.cmp(&first.0));
    stand_ins.truncate(max_drawn.saturating_sub(drawn.len()));
    let drawn: Vec<(usize, u32, u32)> = stand_ins.into_iter().chain(drawn).collect();
    let mut quads = Vec::with_capacity(drawn.len() * QUAD_STRIDE as usize);
    for &key in &drawn {
      quads.extend_from_slice(&self.quad(key, origin, (scale_x, scale_y), p_size));
      if let Some(found) = self.tiles.get_mut(&key) {
        found.last_drawn = self.frame;
      }
    }
    self.ctx.queue.write_buffer(&self.quads, 0, &quads);

    let [red, green, blue, alpha] = self.options.outside.map(f64::from);
    let outside = wgpu::Color { r: red, g: green, b: blue, a: alpha };
    let mut encoder = self.ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("tiles::enc") });
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("tiles::pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: p_target,
          depth_slice: None,
          resolve_target: None,
          ops: wgpu::Operations { load: wgpu::LoadOp::Clear(outside), store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
      });
      pass.set_pipeline(&self.pipeline);
      for (index, key) in drawn.iter().enumerate() {
        if let Some(found) = self.tiles.get(key) {
          pass.set_bind_group(0, &found.bind_group, &[(index as u64 * QUAD_STRIDE) as u32]);
          pass.draw(0..6, 0..1);
        }
      }
    }
    self.ctx.submit(encoder.finish());
    self.evict();
  }

  /// The nearest blurrier tile that is on the GPU and covers `p_tile`'s square, to be drawn until `p_tile` is.
  fn stand_in(&self, p_tile: (usize, u32, u32)) -> Option<(usize, u32, u32)> {
    let (level, tile_x, tile_y) = p_tile;
    // Each level up halves the photo, so the same tile index is half as far along.
    (1..self.pyramid.count.saturating_sub(level))
      .map(|up| (level + up, tile_x >> up, tile_y >> up))
      .find(|key| self.tiles.contains_key(key))
  }

  /// Where `p_tile` goes on a picture of `p_size` pixels that starts at `p_origin` in the photo's pixels, and which part of
  /// the tile's texture goes there, as the bytes of one quad's uniform.
  fn quad(&self, p_tile: (usize, u32, u32), p_origin: (f32, f32), p_scale: (f32, f32), p_size: (u32, u32)) -> Vec<u8> {
    let (level, tile_x, tile_y) = p_tile;
    let step = (1u64 << level) as f32;
    let tile = self.options.tile_size;
    // The tile's own square of the level, with no apron, so neighbouring tiles share exact edges and leave no gap.
    let (left, top) = (tile_x * tile, tile_y * tile);
    let level_dims = self.pyramid.dims(level);
    let (right, bottom) = ((left + tile).min(level_dims.0), (top + tile).min(level_dims.1));
    let clip_x = |level_x: f32| ((level_x * step - p_origin.0) / p_scale.0) / p_size.0 as f32 * 2.0 - 1.0;
    let clip_y = |level_y: f32| 1.0 - ((level_y * step - p_origin.1) / p_scale.1) / p_size.1 as f32 * 2.0;
    // The tile's texture starts one pixel before its square.
    let side = (tile + 2) as f32;
    let to_u = |level_x: u32| (level_x as f32 - left as f32 + 1.0) / side;
    let to_v = |level_y: u32| (level_y as f32 - top as f32 + 1.0) / side;
    let quad = [clip_x(left as f32), clip_y(top as f32), clip_x(right as f32), clip_y(bottom as f32), to_u(left), to_v(top), to_u(right), to_v(bottom)];
    let mut bytes = quad.map(f32::to_ne_bytes).concat();
    bytes.resize(QUAD_STRIDE as usize, 0);
    bytes
  }

  /// Lets go of the tiles not drawn for the longest time while the rest are more than the budget.
  fn evict(&mut self) {
    let each = (self.options.tile_size as u64 + 2) * (self.options.tile_size as u64 + 2) * 4;
    let mut held = self.tiles.len() as u64 * each;
    if held <= self.options.budget_bytes {
      return;
    }
    let mut by_age: Vec<_> = self.tiles.iter().map(|(key, tile)| (tile.last_drawn, *key)).collect();
    by_age.sort();
    for (last_drawn, key) in by_age {
      // Never one this window just drew.
      if held <= self.options.budget_bytes || last_drawn == self.frame {
        break;
      }
      self.tiles.remove(&key);
      held -= each;
    }
  }
}

/// A photo drawn a window at a time through a chain of effects. See the module.
///
/// ```ignore
/// let mut photo = TiledRenderer::new(context, width, height, rgba);
/// // Every frame, for whatever part of the photo is on screen:
/// let view = photo.render(&window, &effects)?;
/// // `photo.snapshot_texture()` is the picture, and `view` the part of it to show (the padding is left out).
/// ```
pub struct TiledRenderer {
  renderer: LiveRenderer,
  cache: TileCache,
}

impl TiledRenderer {
  /// A photo of `p_width` x `p_height` pixels, which is `p_rgba` as RGBA. The pyramid is made here, so this is the call
  /// that takes time on a large photo.
  pub fn new(p_ctx: GpuContext, p_width: u32, p_height: u32, p_rgba: Arc<[u8]>) -> Result<TiledRenderer> {
    Self::with_options(p_ctx, p_width, p_height, p_rgba, TileOptions::default())
  }

  /// Like [`new`](Self::new), cut up, kept and drawn as `p_options` say.
  pub fn with_options(
    p_ctx: GpuContext, p_width: u32, p_height: u32, p_rgba: Arc<[u8]>, p_options: TileOptions,
  ) -> Result<TiledRenderer> {
    if p_width == 0 || p_height == 0 || p_options.tile_size == 0 || p_options.max_drawn == 0 {
      return Err(anyhow!("image, tiles and the number of tiles drawn must not be empty"));
    }
    if p_rgba.len() != p_width as usize * p_height as usize * 4 {
      return Err(anyhow!("expected {} bytes of RGBA, got {}", p_width as usize * p_height as usize * 4, p_rgba.len()));
    }
    Ok(TiledRenderer {
      renderer: LiveRenderer::new(p_ctx.clone()),
      cache: TileCache::new(p_ctx, Pyramid::new(p_width, p_height, p_rgba), p_options),
    })
  }

  /// A photo of `p_width` x `p_height` pixels whose pixels are not held: `p_regions` gives the part of it, at the level, that a
  /// tile needs (see [`Regions`]), cut up, kept and drawn as `p_options` say. Nothing is asked for until a window is drawn.
  pub fn from_regions(
    p_ctx: GpuContext, p_width: u32, p_height: u32, p_regions: Regions, p_options: TileOptions,
  ) -> Result<TiledRenderer> {
    if p_width == 0 || p_height == 0 || p_options.tile_size == 0 || p_options.max_drawn == 0 {
      return Err(anyhow!("image, tiles and the number of tiles drawn must not be empty"));
    }
    Ok(TiledRenderer {
      renderer: LiveRenderer::new(p_ctx.clone()),
      cache: TileCache::new(p_ctx, Pyramid::from_regions(p_width, p_height, p_regions), p_options),
    })
  }

  /// The photo's size in pixels.
  pub fn size(&self) -> (u32, u32) {
    (self.cache.pyramid.width, self.cache.pyramid.height)
  }

  /// Draws `p_window` and runs `p_effects` over it. Returns the part of the picture that is the view, `[x, y, width,
  /// height]` in 0..1 of it: the padding is left out. The picture is [`output_texture`](Self::output_texture).
  ///
  /// Passes that ask for it are told where the picture sits in the photo, so they give the same result at any zoom. The
  /// picture is padded by at least `p_window.pad`, and by as much as the passes say they reach at this zoom (up to
  /// [`TileOptions::max_pad`]), so an effect that looks at its neighbours has them at the edge of the view.
  ///
  /// A pass that is only right at the photo's own size ([`GpuPass::pixel_exact`]) is an error at any other zoom, and the
  /// caller should draw that edit another way.
  pub fn render(&mut self, p_window: &Window, p_effects: &[Vec<GpuPass>]) -> Result<[f32; 4]> {
    if p_window.out_width == 0 || p_window.out_height == 0 || p_window.width <= 0.0 || p_window.height <= 0.0 {
      return Err(anyhow!("the window must not be empty"));
    }
    let scale = p_window.width / p_window.out_width as f32;
    let scale_y = p_window.height / p_window.out_height as f32;
    // A window stretched to another shape draws the photo as it is, but an effect that is told a distance in photo pixels
    // has no one answer for it, so it is not drawn.
    if (scale - scale_y).abs() > 1e-3 * scale.max(scale_y)
      && p_effects.iter().flatten().any(|pass| pass.window || pass.reach > 0.0 || pass.pixel_exact)
    {
      return Err(anyhow!("an effect here needs square pixels, and this window is stretched ({scale} by {scale_y})"));
    }
    if (scale - 1.0).abs() > 1e-3 && p_effects.iter().flatten().any(|pass| pass.pixel_exact && !pass.window) {
      return Err(anyhow!("an effect here is only right at the photo's own size, and this window is at {scale}x"));
    }
    // Each pass reads its neighbours from what the one before wrote, so the reaches add up.
    let reach: f32 = p_effects.iter().flatten().map(|pass| pass.reach).sum();
    let pad = p_window.pad.max((reach / scale).ceil() as u32).min(self.cache.options.max_pad.max(p_window.pad));
    let window = Window { pad, ..*p_window };
    let size = (window.out_width + 2 * pad, window.out_height + 2 * pad);
    let target = self.renderer.source_view(size.0, size.1);
    self.cache.draw(&window, &target, size);
    let (full_width, full_height) = self.size();
    self.renderer.set_window([
      window.x - pad as f32 * scale,
      window.y - pad as f32 * scale_y,
      scale,
      scale_y,
      full_width as f32,
      full_height as f32,
      0.0,
      0.0,
    ]);
    let (full_width, full_height) = (full_width as f32, full_height as f32);
    let origin = (window.x - pad as f32 * scale, window.y - pad as f32 * scale_y);
    let reaches_past = origin.0 < 0.0
      || origin.1 < 0.0
      || origin.0 + size.0 as f32 * scale > full_width
      || origin.1 + size.1 as f32 * scale_y > full_height;
    if reaches_past {
      let mut with_outside = p_effects.to_vec();
      let color: Vec<u8> = self.cache.options.outside.iter().flat_map(|value| value.to_ne_bytes()).collect();
      with_outside.push(vec![GpuPass::new(OUTSIDE_SHADER, color).with_window()]);
      self.renderer.render(&with_outside)?;
    } else {
      self.renderer.render(p_effects)?;
    }
    Ok([
      pad as f32 / size.0 as f32,
      pad as f32 / size.1 as f32,
      window.out_width as f32 / size.0 as f32,
      window.out_height as f32 / size.1 as f32,
    ])
  }

  /// The texture holding the last picture, which the next [`render`](Self::render) draws over.
  pub fn output_texture(&self) -> Option<&wgpu::Texture> {
    self.renderer.output_texture()
  }

  /// A copy of the last picture, which a later render does not draw over.
  pub fn snapshot_texture(&self) -> Result<wgpu::Texture> {
    self.renderer.snapshot_texture()
  }

  /// Waits for the last render and copies the picture, padding included, back as RGBA pixels.
  pub fn read_pixels(&mut self) -> Result<Vec<u8>> {
    self.renderer.read_pixels()
  }

  /// Whether the last [`render`](Self::render) left work for another: a tile drawn blurry until its own is on the GPU (see
  /// [`TileOptions::load_budget`]), or tiles still to put on ahead of time ([`TileOptions::prefetch`]). The caller draws the
  /// same window again, and again, until this is false; every picture on the way is a little sharper.
  pub fn wants_more(&self) -> bool {
    self.cache.wants_more
  }

  /// How many tiles are on the GPU now.
  pub fn tiles_loaded(&self) -> usize {
    self.cache.tiles.len()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A photo whose red is its column, green its row and blue 100, scaled to the full range, so any pixel says where it
  /// came from and any seam between tiles is a jump in a smooth ramp.
  fn ramp(p_width: u32, p_height: u32) -> Arc<[u8]> {
    let mut pixels = Vec::with_capacity(p_width as usize * p_height as usize * 4);
    for y in 0..p_height {
      for x in 0..p_width {
        pixels.extend_from_slice(&[
          (x * 255 / (p_width - 1)) as u8,
          (y * 255 / (p_height - 1)) as u8,
          100,
          255,
        ]);
      }
    }
    pixels.into()
  }

  fn photo(p_tile: u32) -> TiledRenderer {
    photo_with(TileOptions::default().with_tile_size(p_tile))
  }

  /// The renderer with every tile of a window put on in the frame that needs it, so a test is not at the mercy of the clock.
  fn photo_with(p_options: TileOptions) -> TiledRenderer {
    built(p_options.with_load_budget(Duration::MAX))
  }

  /// The renderer with the options exactly as given.
  fn built(p_options: TileOptions) -> TiledRenderer {
    let ctx = GpuContext::new_default_blocking().expect("a GPU");
    TiledRenderer::with_options(ctx, 200, 120, ramp(200, 120), p_options).expect("a renderer")
  }

  #[test]
  fn a_frame_over_its_time_draws_blurrier_tiles_and_asks_to_be_drawn_again() {
    let mut photo = built(TileOptions::default().with_tile_size(32).with_load_budget(Duration::ZERO));
    // The whole photo at a quarter of its size: two tiles of the third level. Nothing is blurrier, so both go on.
    photo.render(&window(0.0, 0.0, 200.0, 120.0, (50, 30), 0), &[]).unwrap();
    assert_eq!(photo.tiles_loaded(), 2);
    assert!(!photo.wants_more(), "the first view has no stand-ins, so every tile is put on");

    // Zoomed in to its own size, the window needs eight tiles. With no time, one goes on and the rest are drawn from the level
    // they are in.
    let view = window(0.0, 0.0, 100.0, 60.0, (100, 60), 0);
    photo.render(&view, &[]).unwrap();
    assert_eq!(photo.tiles_loaded(), 3, "one new tile in a frame with no time");
    assert!(photo.wants_more());
    let blurry = worst_error(&photo.read_pixels().unwrap(), &view, 0);
    assert!(blurry <= 8.0, "a blurry ramp is still the ramp, not a hole: off by {blurry}");

    let mut frames = 1;
    while photo.wants_more() {
      photo.render(&view, &[]).unwrap();
      frames += 1;
      assert!(frames <= 10, "one more tile in each frame finishes in eight");
    }
    assert_eq!(photo.tiles_loaded(), 2 + 8);
    let sharp = worst_error(&photo.read_pixels().unwrap(), &view, 0);
    assert!(sharp <= 2.0, "once every tile is on, it is the photo: off by {sharp}");
  }

  #[test]
  fn the_time_left_puts_on_the_tiles_around_the_window_and_the_sharper_level_under_it() {
    let mut photo = built(TileOptions::default().with_tile_size(32).with_load_budget(Duration::MAX).with_prefetch(1));
    // Two by two tiles of the photo's own size, and the ring of one round them: three by three of the seven by four.
    photo.render(&window(0.0, 0.0, 64.0, 64.0, (64, 64), 0), &[]).unwrap();
    assert_eq!(photo.tiles_loaded(), 9);
    assert!(!photo.wants_more());

    // The whole photo at half its size is four by two tiles of the second level, and under it the whole first level.
    let mut photo = built(TileOptions::default().with_tile_size(32).with_load_budget(Duration::MAX).with_prefetch(1));
    photo.render(&window(0.0, 0.0, 200.0, 120.0, (100, 60), 0), &[]).unwrap();
    assert_eq!(photo.tiles_loaded(), 8 + 28);
    assert!(!photo.wants_more());

    // Without it, only what is drawn is put on.
    let mut photo = built(TileOptions::default().with_tile_size(32).with_load_budget(Duration::MAX));
    photo.render(&window(0.0, 0.0, 64.0, 64.0, (64, 64), 0), &[]).unwrap();
    assert_eq!(photo.tiles_loaded(), 4);
  }

  /// What the ramp is at the photo's position `(x, y)`, which is the middle of a pixel when it ends in .5.
  fn expected(p_x: f32, p_y: f32) -> [f32; 2] {
    [(p_x - 0.5) * 255.0 / 199.0, (p_y - 0.5) * 255.0 / 119.0]
  }

  /// The most any pixel of the picture is from the ramp at the photo position it should show.
  fn worst_error(p_pixels: &[u8], p_window: &Window, p_pad: u32) -> f32 {
    let width = (p_window.out_width + 2 * p_pad) as usize;
    let scale = p_window.width / p_window.out_width as f32;
    let mut worst = 0.0f32;
    for y in 0..p_window.out_height as usize {
      for x in 0..p_window.out_width as usize {
        let at = ((y + p_pad as usize) * width + x + p_pad as usize) * 4;
        let want = expected(p_window.x + (x as f32 + 0.5) * scale, p_window.y + (y as f32 + 0.5) * scale);
        // The edges of the photo are held, so the ramp stops there.
        let want = [want[0].clamp(0.0, 255.0), want[1].clamp(0.0, 255.0)];
        worst = worst.max((p_pixels[at] as f32 - want[0]).abs()).max((p_pixels[at + 1] as f32 - want[1]).abs());
      }
    }
    worst
  }

  fn window(p_x: f32, p_y: f32, p_width: f32, p_height: f32, p_out: (u32, u32), p_pad: u32) -> Window {
    Window { x: p_x, y: p_y, width: p_width, height: p_height, out_width: p_out.0, out_height: p_out.1, pad: p_pad }
  }

  #[test]
  fn the_whole_photo_at_its_own_size_is_the_photo_with_no_seams_between_tiles() {
    let mut photo = photo(64);
    let view = window(0.0, 0.0, 200.0, 120.0, (200, 120), 0);
    photo.render(&view, &[]).unwrap();
    let pixels = photo.read_pixels().unwrap();
    assert_eq!(pixels, ramp(200, 120).to_vec().iter().copied().collect::<Vec<u8>>().as_slice(), "every pixel is the photo's");
  }

  #[test]
  fn a_smaller_picture_comes_from_a_smaller_level() {
    let mut photo = photo(64);
    for (out, scale_label) in [((100, 60), "half"), ((67, 40), "a third"), ((50, 30), "a quarter")] {
      let view = window(0.0, 0.0, 200.0, 120.0, out, 0);
      photo.render(&view, &[]).unwrap();
      let pixels = photo.read_pixels().unwrap();
      let error = worst_error(&pixels, &view, 0);
      assert!(error <= 3.0, "{scale_label}: the ramp is off by {error}");
    }
  }

  #[test]
  fn a_zoomed_in_window_shows_that_part_of_the_photo_smoothly() {
    let mut photo = photo(64);
    // Across tile edges (64 and 128), magnified four times.
    let view = window(50.0, 40.0, 100.0, 60.0, (400, 240), 0);
    photo.render(&view, &[]).unwrap();
    let pixels = photo.read_pixels().unwrap();
    let error = worst_error(&pixels, &view, 0);
    assert!(error <= 2.0, "the ramp is off by {error}");
    // 50..150 across touches three columns of 64 and 40..100 down two rows, of the four by two the photo has.
    assert_eq!(photo.tiles_loaded(), 6, "only the tiles in the window are loaded");
  }

  #[test]
  fn padding_is_more_of_the_photo_all_the_way_round() {
    let mut photo = photo(64);
    let view = window(60.0, 30.0, 80.0, 48.0, (160, 96), 8);
    photo.render(&view, &[]).unwrap();
    let padded = photo.read_pixels().unwrap();
    assert_eq!(padded.len(), (160 + 16) * (96 + 16) * 4);
    let error = worst_error(&padded, &view, 8);
    assert!(error <= 2.0, "the ramp is off by {error}");
    // The padding itself is the photo around the view, not black.
    let corner = &padded[..4];
    let want = expected(60.0 - 8.0 * 0.5 + 0.25, 30.0 - 8.0 * 0.5 + 0.25);
    assert!((corner[0] as f32 - want[0]).abs() <= 2.0 && (corner[1] as f32 - want[1]).abs() <= 2.0);
  }

  #[test]
  fn nothing_is_drawn_where_the_window_is_past_the_photo() {
    let mut photo = photo(64);
    // The left half of the window is left of the photo.
    let view = window(-100.0, 0.0, 200.0, 120.0, (200, 120), 0);
    photo.render(&view, &[]).unwrap();
    let pixels = photo.read_pixels().unwrap();
    assert_eq!(&pixels[..4], &[0, 0, 0, 0], "outside the photo is clear");
    let inside = (60 * 200 + 150) * 4;
    assert_eq!(pixels[inside + 3], 255, "inside the photo is drawn");
  }

  #[test]
  fn a_pass_that_asks_is_told_where_the_picture_is_in_the_photo() {
    // Writes the photo position of each pixel into red and green (a tenth of a photo pixel per step, up to 25).
    const WHERE: &str = r#"
      struct Window { origin: vec2<f32>, scale: f32, pad: f32, full: vec2<f32>, pad2: vec2<f32> };
      @group(0) @binding(0) var input_tex: texture_2d<f32>;
      @group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
      @group(0) @binding(5) var<uniform> window: Window;
      @compute @workgroup_size(8, 8)
      fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
        let dims = textureDimensions(input_tex);
        if (gid.x >= dims.x || gid.y >= dims.y) { return; }
        let at = window.origin + (vec2<f32>(gid.xy) + 0.5) * window.scale;
        textureStore(output_tex, vec2<i32>(gid.xy), vec4<f32>(at / window.full, 0.0, 1.0));
      }
    "#;
    let mut photo = photo(64);
    let view = window(40.0, 30.0, 100.0, 60.0, (100, 60), 4);
    photo.render(&view, &[vec![GpuPass::new(WHERE, Vec::new()).with_window()]]).unwrap();
    let pixels = photo.read_pixels().unwrap();
    let width = 108usize;
    for (x, y) in [(0usize, 0usize), (50, 30), (107, 67)] {
      let at = (y * width + x) * 4;
      let photo_x = 40.0 - 4.0 + (x as f32 + 0.5);
      let photo_y = 30.0 - 4.0 + (y as f32 + 0.5);
      assert!((pixels[at] as f32 - photo_x / 200.0 * 255.0).abs() <= 1.0, "x at {x},{y}");
      assert!((pixels[at + 1] as f32 - photo_y / 120.0 * 255.0).abs() <= 1.0, "y at {x},{y}");
    }
  }

  #[test]
  fn the_options_decide_the_outside_the_filter_and_how_many_tiles_are_drawn() {
    // Outside the photo is the color asked for.
    let mut photo = photo_with(TileOptions::default().with_tile_size(64).with_outside([0.0, 0.0, 0.0, 1.0]));
    photo.render(&window(-100.0, 0.0, 200.0, 120.0, (200, 120), 0), &[]).unwrap();
    assert_eq!(&photo.read_pixels().unwrap()[..4], &[0, 0, 0, 255]);

    // Nearest shows the photo's own pixels when magnified: four screen pixels show one photo pixel, so they are the same
    // where a smooth lookup would change on every one.
    let mut photo = photo_with(TileOptions::default().with_tile_size(64).with_filter(wgpu::FilterMode::Nearest));
    photo.render(&window(100.0, 0.0, 50.0, 30.0, (200, 120), 0), &[]).unwrap();
    let pixels = photo.read_pixels().unwrap();
    assert_eq!(pixels[0], pixels[4], "four screen pixels show one photo pixel");
    assert_eq!(pixels[4], pixels[8]);
    assert_eq!(pixels[8], pixels[12]);

    // Fewer tiles drawn than the window needs leaves the rest out.
    let mut photo = photo_with(TileOptions::default().with_tile_size(64).with_max_drawn(1));
    photo.render(&window(0.0, 0.0, 200.0, 120.0, (200, 120), 0), &[]).unwrap();
    assert_eq!(photo.tiles_loaded(), 1);

    // A sharper bias picks a sharper level for the same window, so a shrunk ramp is still right.
    let mut photo = photo_with(TileOptions::default().with_tile_size(64).with_level_bias(-1.0));
    let view = window(0.0, 0.0, 200.0, 120.0, (50, 30), 0);
    photo.render(&view, &[]).unwrap();
    assert!(worst_error(&photo.read_pixels().unwrap(), &view, 0) <= 3.0);

    // Nothing to cut up or draw is refused.
    let ctx = GpuContext::new_default_blocking().unwrap();
    assert!(TiledRenderer::with_options(ctx, 200, 120, ramp(200, 120), TileOptions::default().with_tile_size(0)).is_err());
  }

  /// A pass that writes the pixel it reads, for testing what the renderer does around a pass.
  const COPY: &str = r#"
    @group(0) @binding(0) var input_tex: texture_2d<f32>;
    @group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
    @compute @workgroup_size(8, 8)
    fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
      let dims = textureDimensions(input_tex);
      if (gid.x >= dims.x || gid.y >= dims.y) { return; }
      textureStore(output_tex, vec2<i32>(gid.xy), textureLoad(input_tex, vec2<i32>(gid.xy), 0));
    }
  "#;

  #[test]
  fn a_window_in_another_shape_shows_the_photo_stretched_and_refuses_effects_that_need_square_pixels() {
    let mut photo = photo(64);
    // The whole photo in a picture half as wide and as tall as it: one pixel across is two photo pixels, one down is one.
    let view = window(0.0, 0.0, 200.0, 120.0, (100, 120), 0);
    photo.render(&view, &[]).unwrap();
    let pixels = photo.read_pixels().unwrap();
    for (x, y) in [(0usize, 0usize), (30, 40), (99, 119), (50, 60)] {
      let want = expected((x as f32 + 0.5) * 2.0, y as f32 + 0.5);
      let at = (y * 100 + x) * 4;
      assert!((pixels[at] as f32 - want[0].clamp(0.0, 255.0)).abs() <= 3.0, "red at {x},{y}");
      assert!((pixels[at + 1] as f32 - want[1].clamp(0.0, 255.0)).abs() <= 3.0, "green at {x},{y}");
    }
    let aware = vec![vec![GpuPass::new(COPY, Vec::new()).with_window()]];
    let error = photo.render(&view, &aware).unwrap_err();
    assert!(error.to_string().contains("square pixels"), "{error}");
  }

  #[test]
  fn a_pass_that_is_only_right_at_the_photo_s_own_size_is_refused_at_any_other_zoom() {
    let mut photo = photo(64);
    let exact = vec![vec![GpuPass::new(COPY, Vec::new()).with_pixel_exact()]];
    assert!(photo.render(&window(0.0, 0.0, 200.0, 120.0, (200, 120), 0), &exact).is_ok(), "at its own size it is right");
    let error = photo.render(&window(0.0, 0.0, 200.0, 120.0, (100, 60), 0), &exact).unwrap_err();
    assert!(error.to_string().contains("only right at the photo's own size"), "{error}");
    // One that handles the window itself is fine at any zoom.
    let aware = vec![vec![GpuPass::new(COPY, Vec::new()).with_pixel_exact().with_window()]];
    assert!(photo.render(&window(0.0, 0.0, 200.0, 120.0, (100, 60), 0), &aware).is_ok());
  }

  #[test]
  fn what_the_passes_reach_pads_the_picture_up_to_the_most_the_options_allow() {
    // Two passes that each read 20 photo pixels away, at two photo pixels to a screen pixel: 10 screen pixels each.
    let reaching = vec![vec![GpuPass::new(COPY, Vec::new()).with_reach(20.0), GpuPass::new(COPY, Vec::new()).with_reach(20.0)]];
    let view = window(0.0, 0.0, 200.0, 120.0, (100, 60), 0);
    let mut photo = photo(64);
    let part = photo.render(&view, &reaching).unwrap();
    assert!((part[0] - 20.0 / 140.0).abs() < 1e-6, "20 pixels of padding each side of 100: {part:?}");
    assert!((part[2] - 100.0 / 140.0).abs() < 1e-6);
    // A pad asked for in the window is a floor, not a replacement.
    let part = photo.render(&window(0.0, 0.0, 200.0, 120.0, (100, 60), 30), &reaching).unwrap();
    assert!((part[0] - 30.0 / 160.0).abs() < 1e-6, "{part:?}");
    // The options cap it.
    let mut limited = photo_with(TileOptions::default().with_tile_size(64).with_max_pad(4));
    let part = limited.render(&view, &reaching).unwrap();
    assert!((part[0] - 4.0 / 108.0).abs() < 1e-6, "{part:?}");
  }

  #[test]
  fn tiles_not_drawn_lately_are_let_go_when_there_are_too_many() {
    let mut photo = photo_with(TileOptions::default().with_tile_size(32).with_budget_bytes(4 * 34 * 34 * 4));
    photo.render(&window(0.0, 0.0, 200.0, 120.0, (200, 120), 0), &[]).unwrap();
    assert!(photo.tiles_loaded() > 4, "a whole photo needs more than the budget");
    photo.render(&window(0.0, 0.0, 32.0, 32.0, (64, 64), 0), &[]).unwrap();
    assert!(photo.tiles_loaded() <= 5, "the tiles of the first window went, leaving about the budget: {}", photo.tiles_loaded());
  }
  #[test]
  fn a_photo_made_by_a_function_is_asked_only_for_the_tiles_in_view_and_draws_as_the_photo() {
    let asked = Arc::new(std::sync::Mutex::new(Vec::new()));
    let ramp_of = ramp(200, 120);
    let regions: Regions = {
      let asked = Arc::clone(&asked);
      Arc::new(move |level, left, top, width, height| {
        asked.lock().unwrap().push((level, left, top, width, height));
        assert_eq!(level, 0, "a window at the photo's own size is made from level 0");
        let mut out = Vec::new();
        for y in 0..height as i32 {
          for x in 0..width as i32 {
            let (source_x, source_y) = ((left + x).clamp(0, 199) as usize, (top + y).clamp(0, 119) as usize);
            out.extend_from_slice(&ramp_of[(source_y * 200 + source_x) * 4..(source_y * 200 + source_x) * 4 + 4]);
          }
        }
        out
      })
    };
    let ctx = GpuContext::new_default_blocking().expect("a GPU");
    let options = TileOptions::default().with_tile_size(64).with_load_budget(Duration::MAX);
    let mut photo = TiledRenderer::from_regions(ctx, 200, 120, regions, options).expect("a renderer");
    // The middle of the photo at its own size: columns 1 and 2 of tiles of 64 and rows 0 and 1, not the whole photo.
    let view = window(70.0, 20.0, 100.0, 60.0, (100, 60), 0);
    photo.render(&view, &[]).unwrap();
    assert_eq!(asked.lock().unwrap().len(), photo.tiles_loaded());
    assert_eq!(photo.tiles_loaded(), 4, "two columns and two rows of 64: 70..170 and 20..80");
    assert!(worst_error(&photo.read_pixels().unwrap(), &view, 0) <= 2.0, "it is the ramp");
    // Every tile is one pixel more than its square on each side, from the level it is in.
    assert!(asked.lock().unwrap().iter().all(|&(_, _, _, width, height)| (width, height) == (66, 66)));
    assert_eq!(photo.size(), (200, 120));
  }
}
