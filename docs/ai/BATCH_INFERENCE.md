# Batch Inference for Super-Resolution

## Overview

Batch inference processes multiple tiles in a single model forward pass, which can provide:
- **1.5-3× speedup** due to better GPU/CPU utilization
- **Reduced overhead** from fewer kernel launches and memory transfers
- **Better parallelization** within the neural network

---

## How It Works

### Current: Sequential Inference
```
Tile 1 → Model → Output 1
Tile 2 → Model → Output 2
Tile 3 → Model → Output 3
Tile 4 → Model → Output 4
Total: 4 × inference_time
```

### Batch Inference
```
[Tile 1, Tile 2, Tile 3, Tile 4] → Model → [Output 1, Output 2, Output 3, Output 4]
Total: 1 × batch_inference_time (faster than 4×)
```

---

## Requirements

### 1. Model Must Support Dynamic Batch Size

Check if the model has a dynamic batch dimension:

```python
import onnx

model = onnx.load("SCUNet-GAN.onnx")
input_shape = model.graph.input[0].type.tensor_type.shape

for i, dim in enumerate(input_shape.dim):
    if dim.dim_param:  # Dynamic dimension (named)
        print(f"Dimension {i}: dynamic ({dim.dim_param})")
    else:  # Fixed dimension
        print(f"Dimension {i}: fixed ({dim.dim_value})")
```

Expected output for batch support:
```
Dimension 0: dynamic (batch) or fixed (1)  ← Need this to be dynamic
Dimension 1: fixed (3)                      ← Channels
Dimension 2: dynamic (height) or fixed     ← Height
Dimension 3: dynamic (width) or fixed      ← Width
```

### 2. Re-export with Dynamic Batch (if needed)

```python
import torch

# Load your model
model = load_scunet_model()
model.eval()

# Export with dynamic batch dimension
dummy_input = torch.randn(1, 3, 256, 256)
torch.onnx.export(
    model,
    dummy_input,
    "SCUNet-GAN-batch.onnx",
    opset_version=16,
    input_names=['input'],
    output_names=['output'],
    dynamic_axes={
        'input': {0: 'batch', 2: 'height', 3: 'width'},
        'output': {0: 'batch', 2: 'height', 3: 'width'}
    }
)
```

---

## Implementation

### Step 1: Collect Tiles into Batches

```rust
/// Collects tiles into batches for efficient inference.
fn collect_batches(
    input_image: &Image,
    tile_size: u32,
    overlap: u32,
    batch_size: usize,
) -> Vec<TileBatch> {
    let (orig_w, orig_h) = input_image.dimensions::<u32>();
    let stride = tile_size - overlap;

    let tiles_x = ((orig_w + stride - 1) / stride).max(1);
    let tiles_y = ((orig_h + stride - 1) / stride).max(1);

    let mut all_tiles = Vec::new();

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let mut x_start = tx * stride;
            let mut y_start = ty * stride;

            // Align last tile to edge
            if x_start + tile_size > orig_w && orig_w > tile_size {
                x_start = orig_w - tile_size;
            }
            if y_start + tile_size > orig_h && orig_h > tile_size {
                y_start = orig_h - tile_size;
            }

            all_tiles.push(TileInfo {
                x: x_start,
                y: y_start,
                width: tile_size.min(orig_w - x_start),
                height: tile_size.min(orig_h - y_start),
            });
        }
    }

    // Group into batches
    all_tiles
        .chunks(batch_size)
        .map(|chunk| TileBatch {
            tiles: chunk.to_vec(),
        })
        .collect()
}

struct TileInfo {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

struct TileBatch {
    tiles: Vec<TileInfo>,
}
```

### Step 2: Batch Preprocessing

```rust
fn preprocess_batch(&self, images: &[Image]) -> ndarray::Array4<f32> {
    let batch_size = images.len();
    let (width, height) = images[0].dimensions::<u32>();
    let total_pixels = (width * height) as usize;

    // Shape: [batch, channels=3, height, width]
    let mut values = Vec::with_capacity(batch_size * 3 * total_pixels);

    for image in images {
        // Each image: CHW layout
        for channel in 0..3 {
            for y in 0..height {
                for x in 0..width {
                    if let Some((r, g, b, _)) = image.get_pixel(x, y) {
                        let val = match channel {
                            0 => r as f32 / 255.0,
                            1 => g as f32 / 255.0,
                            2 => b as f32 / 255.0,
                            _ => 0.0,
                        };
                        values.push(val);
                    } else {
                        values.push(0.0);
                    }
                }
            }
        }
    }

    ndarray::Array4::from_shape_vec(
        (batch_size, 3, height as usize, width as usize),
        values
    ).expect("Failed to create batch array")
}
```

### Step 3: Batch Inference

```rust
fn infer_batch(&mut self, tiles: &[Image]) -> Vec<(u32, u32, Vec<f32>)> {
    let batch_size = tiles.len();

    // Preprocess all tiles into a single batch
    let input_array = self.preprocess_batch(tiles);
    let input_array = input_array.as_standard_layout().into_owned();

    let shape = input_array.shape();
    let data = input_array.as_slice().expect("Failed to get contiguous array");
    let input_value = TensorRef::from_array_view((shape, data))
        .expect("Failed to create tensor ref");

    // Run batch inference
    let mut session = self.session.lock().expect("Failed to lock session");
    let outputs = session.run(ort::inputs![input_value])
        .expect("Failed to run inference");

    let output = &outputs[0];
    let (out_shape, out_view) = output
        .try_extract_tensor::<f32>()
        .expect("Failed to extract tensor");

    // out_shape: [batch, 3, height, width]
    let out_batch = out_shape[0] as usize;
    let out_height = out_shape[2] as u32;
    let out_width = out_shape[3] as u32;
    let pixels_per_image = (out_height * out_width * 3) as usize;

    // Split output back into individual tile results
    let mut results = Vec::with_capacity(batch_size);
    for i in 0..out_batch {
        let start = i * pixels_per_image;
        let end = start + pixels_per_image;
        let tile_data: Vec<f32> = out_view[start..end].to_vec();
        results.push((out_width, out_height, tile_data));
    }

    results
}
```

### Step 4: Updated Enhance Function

```rust
pub fn enhance_batched(&mut self, input_image: Image, batch_size: usize) -> Image {
    let start = Instant::now();
    let (orig_w, orig_h) = input_image.dimensions::<u32>();
    let tile_size = self.tile_size as u32;
    let overlap = self.overlap as u32;

    println!("Processing {}x{} with batch size {}", orig_w, orig_h, batch_size);

    // Collect all tile positions
    let batches = collect_batches(&input_image, tile_size, overlap, batch_size);
    let total_batches = batches.len();

    // Initialize accumulators
    let result_w = orig_w;
    let result_h = orig_h;
    let num_pixels = (result_w * result_h) as usize;
    let mut sum_r = vec![0.0f32; num_pixels];
    let mut sum_g = vec![0.0f32; num_pixels];
    let mut sum_b = vec![0.0f32; num_pixels];
    let mut weights = vec![0.0f32; num_pixels];

    for (batch_idx, batch) in batches.iter().enumerate() {
        println!("Batch {}/{} ({} tiles)", batch_idx + 1, total_batches, batch.tiles.len());

        // Extract tile images
        let tile_images: Vec<Image> = batch.tiles.iter()
            .map(|t| cropped(&input_image, t.x, t.y, t.width, t.height))
            .collect();

        // Run batch inference
        let outputs = self.model.infer_batch(&tile_images);

        // Accumulate results
        for (tile_info, (out_w, out_h, float_data)) in batch.tiles.iter().zip(outputs) {
            let hw = (out_w * out_h) as usize;

            for py in 0..out_h {
                for px in 0..out_w {
                    let dest_x = tile_info.x + px;
                    let dest_y = tile_info.y + py;

                    if dest_x < result_w && dest_y < result_h {
                        let dest_idx = (dest_y * result_w + dest_x) as usize;
                        let src_idx = (py * out_w + px) as usize;

                        sum_r[dest_idx] += float_data[src_idx];
                        sum_g[dest_idx] += float_data[hw + src_idx];
                        sum_b[dest_idx] += float_data[2 * hw + src_idx];
                        weights[dest_idx] += 1.0;
                    }
                }
            }
        }
    }

    println!("Batch processing complete in {:?}", start.elapsed());

    // Create final image
    let mut result = Image::new(result_w, result_h);
    let mut rgba_data = vec![0u8; num_pixels * 4];

    for i in 0..num_pixels {
        let w = weights[i];
        if w > 0.0 {
            let r = (sum_r[i] / w).clamp(0.0, 1.0);
            let g = (sum_g[i] / w).clamp(0.0, 1.0);
            let b = (sum_b[i] / w).clamp(0.0, 1.0);

            rgba_data[i * 4] = (r * 255.0) as u8;
            rgba_data[i * 4 + 1] = (g * 255.0) as u8;
            rgba_data[i * 4 + 2] = (b * 255.0) as u8;
            rgba_data[i * 4 + 3] = 255;
        }
    }

    result.set_new_pixels(&rgba_data, result_w, result_h);
    result
}
```

---

## Optimal Batch Sizes

| Hardware         | Recommended Batch Size | Notes                       |
| ---------------- | ---------------------- | --------------------------- |
| CPU (8 cores)    | 2-4                    | Limited by memory bandwidth |
| CPU (16+ cores)  | 4-8                    | Better parallelization      |
| GPU (4GB VRAM)   | 4-8                    | ~200MB per 256×256 tile     |
| GPU (8GB VRAM)   | 8-16                   | More headroom               |
| GPU (16GB+ VRAM) | 16-32                  | Maximum throughput          |

### Memory Calculation

For SCUNet-GAN with 256×256 tiles:
```
Per tile memory ≈ 256 × 256 × 3 × 4 bytes × (input + intermediate + output)
                ≈ 50-200 MB depending on model architecture

Batch of 4: ~200-800 MB
Batch of 8: ~400-1600 MB
```

---

## Performance Expectations

### CPU Batch Performance

| Batch Size  | Time per Batch | Tiles/sec | Speedup |
| ----------- | -------------- | --------- | ------- |
| 1 (current) | 2.0s           | 0.5       | 1.0×    |
| 2           | 3.2s           | 0.625     | 1.25×   |
| 4           | 5.5s           | 0.73      | 1.46×   |
| 8           | 10s            | 0.8       | 1.6×    |

*CPU batch speedup is limited due to memory bandwidth*

### GPU Batch Performance

| Batch Size | Time per Batch | Tiles/sec | Speedup |
| ---------- | -------------- | --------- | ------- |
| 1          | 0.15s          | 6.7       | 1.0×    |
| 4          | 0.25s          | 16.0      | 2.4×    |
| 8          | 0.40s          | 20.0      | 3.0×    |
| 16         | 0.70s          | 22.9      | 3.4×    |

*GPU benefits more from batching due to parallel execution*

---

## Limitations

### 1. All Tiles Must Be Same Size
Batching requires uniform tensor shapes. For edge tiles that are smaller:

```rust
// Option A: Pad smaller tiles to tile_size
fn pad_to_size(image: &Image, target_w: u32, target_h: u32) -> Image {
    let (w, h) = image.dimensions::<u32>();
    if w == target_w && h == target_h {
        return image.clone();
    }

    let mut padded = Image::new(target_w, target_h);
    // Copy original image
    padded.draw(image, 0, 0);
    // Pad with edge pixels (replicate padding)
    // ...
    padded
}

// Option B: Process edge tiles separately with batch_size=1
```

### 2. Memory Constraints
Large batches may cause out-of-memory errors:

```rust
fn determine_batch_size(tile_size: u32, available_memory_mb: usize) -> usize {
    let tile_memory_mb = (tile_size * tile_size * 3 * 4 * 3) as usize / (1024 * 1024);
    let max_batch = available_memory_mb / tile_memory_mb;
    max_batch.clamp(1, 16)
}
```

### 3. Diminishing Returns
Beyond a certain batch size, speedup plateaus:

```
Batch 1→2:  ~25% faster
Batch 2→4:  ~15% faster
Batch 4→8:  ~10% faster
Batch 8→16: ~5% faster
```

---

## Builder API Extension

```rust
impl<B: Backend> SuperResolutionBuilder<B> {
    /// Set batch size for inference (default: 1)
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size.clamp(1, 32);
        self
    }
}

// Usage
let sr = SuperResolutionBuilder::new(device)
    .with_tile_size(256)
    .with_overlap(64)
    .with_batch_size(4)  // Process 4 tiles at once
    .build();
```

---

## Combining with Other Optimizations

Batch inference works well with:

| Optimization      | Combined Benefit      |
| ----------------- | --------------------- |
| GPU inference     | 20-100× total speedup |
| INT8 quantization | 3-6× total speedup    |
| Smaller model     | 4-8× total speedup    |

Example combined setup:
```rust
// GPU + Batch + Quantized = Maximum speed
let sr = SuperResolutionBuilder::new(device)
    .with_model(ModelType::NAFNetInt8)
    .with_gpu(true)
    .with_batch_size(8)
    .build();
```

---

## Next Steps

1. Re-export SCUNet-GAN with dynamic batch dimension
2. Implement `preprocess_batch` and `infer_batch` methods
3. Add batch size configuration to builder
4. Benchmark various batch sizes on target hardware
5. Implement automatic batch size selection based on available memory
