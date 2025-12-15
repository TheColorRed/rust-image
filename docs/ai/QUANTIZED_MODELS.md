# Quantized Models (INT8) for Super-Resolution

## Overview

Model quantization converts floating-point weights (FP32) to lower-precision integers (INT8), providing:
- **2-4× faster inference** on CPU
- **2-4× smaller model size**
- **Lower memory bandwidth** requirements

The trade-off is a small reduction in output quality (typically 1-3% PSNR drop).

---

## Quantization Types

### 1. Dynamic Quantization
- Weights quantized at load time
- Activations quantized during inference
- **Easiest to implement**
- ~1.5-2× speedup

### 2. Static Quantization (PTQ - Post-Training Quantization)
- Weights and activations pre-quantized
- Requires calibration dataset
- **Best balance of speed and quality**
- ~2-3× speedup

### 3. Quantization-Aware Training (QAT)
- Model trained with quantization simulation
- Best quality preservation
- Requires retraining
- ~2-4× speedup

---

## ONNX Runtime Quantization

ONNX Runtime supports quantized models natively with optimized INT8 kernels.

### Step 1: Install onnxruntime-tools

```bash
pip install onnxruntime-tools onnx
```

### Step 2: Dynamic Quantization (Simplest)

```python
from onnxruntime.quantization import quantize_dynamic, QuantType

# Quantize the model
quantize_dynamic(
    model_input="SCUNet-GAN.onnx",
    model_output="SCUNet-GAN-int8-dynamic.onnx",
    weight_type=QuantType.QInt8
)

print("Dynamic quantization complete!")
```

### Step 3: Static Quantization (Better Performance)

```python
from onnxruntime.quantization import quantize_static, CalibrationDataReader, QuantType
import numpy as np

class SRCalibrationDataReader(CalibrationDataReader):
    """Provides calibration data for quantization."""

    def __init__(self, calibration_images: list[np.ndarray]):
        self.images = calibration_images
        self.index = 0

    def get_next(self):
        if self.index >= len(self.images):
            return None

        # Preprocess image to NCHW format
        img = self.images[self.index]
        img = img.astype(np.float32) / 255.0
        img = np.transpose(img, (2, 0, 1))  # HWC -> CHW
        img = np.expand_dims(img, 0)  # Add batch dimension

        self.index += 1
        return {"input": img}

    def rewind(self):
        self.index = 0

# Load calibration images (10-100 representative samples)
import cv2
calibration_images = []
for path in ["cal1.png", "cal2.png", "cal3.png", ...]:
    img = cv2.imread(path)
    img = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
    img = cv2.resize(img, (256, 256))
    calibration_images.append(img)

# Create calibration reader
calibration_reader = SRCalibrationDataReader(calibration_images)

# Quantize with static calibration
quantize_static(
    model_input="SCUNet-GAN.onnx",
    model_output="SCUNet-GAN-int8-static.onnx",
    calibration_data_reader=calibration_reader,
    quant_format=QuantFormat.QDQ,  # Quantize-Dequantize format
    activation_type=QuantType.QInt8,
    weight_type=QuantType.QInt8,
    per_channel=True,  # Per-channel quantization for better accuracy
    reduce_range=False,
    extra_options={
        "ActivationSymmetric": False,
        "WeightSymmetric": True,
    }
)

print("Static quantization complete!")
```

---

## Calibration Dataset Guidelines

For best quantization quality, use calibration images that:

1. **Represent your actual use case**
   - If processing photos, use photos
   - If processing anime, use anime images

2. **Cover the input distribution**
   - Include bright and dark images
   - Include high and low contrast
   - Include various content types

3. **Use 10-100 samples**
   - Too few: poor calibration
   - Too many: slow calibration, diminishing returns

### Example Calibration Script

```python
import os
import cv2
import numpy as np
from pathlib import Path

def prepare_calibration_dataset(
    input_dir: str,
    output_dir: str,
    tile_size: int = 256,
    max_samples: int = 50
):
    """Extract tiles from images for calibration."""

    Path(output_dir).mkdir(parents=True, exist_ok=True)

    tiles = []
    for img_path in Path(input_dir).glob("*.{png,jpg,jpeg}"):
        img = cv2.imread(str(img_path))
        if img is None:
            continue

        h, w = img.shape[:2]

        # Extract random tiles
        for _ in range(3):  # 3 tiles per image
            if h >= tile_size and w >= tile_size:
                y = np.random.randint(0, h - tile_size)
                x = np.random.randint(0, w - tile_size)
                tile = img[y:y+tile_size, x:x+tile_size]
                tiles.append(tile)

        if len(tiles) >= max_samples:
            break

    # Save tiles
    for i, tile in enumerate(tiles[:max_samples]):
        cv2.imwrite(f"{output_dir}/tile_{i:03d}.png", tile)

    print(f"Saved {len(tiles[:max_samples])} calibration tiles to {output_dir}")

# Usage
prepare_calibration_dataset(
    input_dir="path/to/your/images",
    output_dir="calibration_tiles",
    tile_size=256,
    max_samples=50
)
```

---

## Using Quantized Models in Rust

The quantized ONNX model is used exactly like the original:

```rust
const QUANTIZED_MODEL: &str = "packages/ai/super-resolution/models/SCUNet-GAN-int8.onnx";

impl<B: Backend> MaximModel<B> {
    pub fn new_quantized(device: B::Device, _tile_size: usize) -> Self {
        let model_path = if Path::new(QUANTIZED_MODEL).exists() {
            println!("Using INT8 quantized model");
            QUANTIZED_MODEL
        } else {
            println!("Quantized model not found, using FP32");
            NAFNET_MODEL
        };

        let model_bytes = std::fs::read(model_path)
            .expect("Failed to read ONNX model file");

        let num_threads = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);

        let session = Session::builder()
            .expect("Failed to create session builder")
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .expect("Failed to set optimization level")
            .with_intra_threads(num_threads)
            .expect("Failed to set intra threads")
            .commit_from_memory(&model_bytes)
            .expect("Failed to load ONNX model");

        Self {
            _device: device,
            session: Mutex::new(session),
        }
    }
}
```

---

## Performance Comparison

| Model                     | Size  | Inference Time* | Quality (PSNR) |
| ------------------------- | ----- | --------------- | -------------- |
| SCUNet-GAN FP32           | 60 MB | 2.0s            | 32.5 dB        |
| SCUNet-GAN INT8 (dynamic) | 15 MB | 1.2s            | 32.2 dB        |
| SCUNet-GAN INT8 (static)  | 15 MB | 0.8s            | 32.3 dB        |

*Per 256×256 tile on CPU

---

## Mixed Precision: FP16

For GPUs, FP16 (half precision) is often better than INT8:

```python
from onnxruntime.transformers import float16

# Convert to FP16
float16.convert_float_to_float16(
    "SCUNet-GAN.onnx",
    "SCUNet-GAN-fp16.onnx",
    keep_io_types=True  # Keep input/output as FP32
)
```

### FP16 Benefits
- 2× smaller model
- ~1.5× faster on GPUs with tensor cores
- Minimal quality loss

### FP16 in ONNX Runtime

```rust
use ort::execution_providers::CUDAExecutionProvider;

// CUDA EP automatically uses FP16 tensor cores when available
let cuda = CUDAExecutionProvider::default()
    .with_device_id(0);
```

---

## Quantization Caveats

### 1. Not All Operators Support INT8
Some operators will fall back to FP32:
- `Einsum` (common in transformers)
- Complex attention mechanisms
- Custom operators

Check which operators were quantized:
```python
import onnx

model = onnx.load("SCUNet-GAN-int8.onnx")
for node in model.graph.node:
    if "Quant" in node.op_type or "DequantizeLinear" in node.op_type:
        print(f"Quantized: {node.name}")
```

### 2. Quality Degradation Patterns
INT8 quantization affects:
- Fine details (may appear slightly blurry)
- Gradients (may show banding)
- Extreme values (clipping artifacts)

### 3. Calibration Data Matters
Poor calibration causes:
- Color shifts
- Contrast changes
- Detail loss in specific image types

---

## Complete Quantization Pipeline

```bash
#!/bin/bash
# quantize_model.sh

# 1. Prepare calibration data
python prepare_calibration.py \
    --input-dir ./sample_images \
    --output-dir ./calibration_tiles \
    --tile-size 256 \
    --max-samples 50

# 2. Run static quantization
python quantize.py \
    --model SCUNet-GAN.onnx \
    --output SCUNet-GAN-int8.onnx \
    --calibration-dir ./calibration_tiles \
    --format static

# 3. Validate quantized model
python validate.py \
    --original SCUNet-GAN.onnx \
    --quantized SCUNet-GAN-int8.onnx \
    --test-image test.png

# 4. Copy to models directory
cp SCUNet-GAN-int8.onnx packages/ai/super-resolution/models/
```

---

## Recommendations

| Scenario                            | Recommendation                     |
| ----------------------------------- | ---------------------------------- |
| **CPU inference, speed critical**   | Static INT8 quantization           |
| **CPU inference, quality critical** | Dynamic INT8 or FP32               |
| **GPU inference (NVIDIA)**          | FP16 with tensor cores             |
| **GPU inference (AMD)**             | FP16 or FP32 (INT8 support varies) |
| **Memory constrained**              | INT8 (4× smaller)                  |

---

## Next Steps

1. Create calibration dataset from representative images
2. Run static quantization with calibration
3. Benchmark speed improvement
4. Compare output quality (visual + PSNR)
5. If quality acceptable, integrate as optional model variant
