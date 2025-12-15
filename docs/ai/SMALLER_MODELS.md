# Smaller/Faster Models for Super-Resolution

## Overview

The current SCUNet-GAN model is a high-quality but slow image restoration model. Switching to a smaller, faster model could provide **2-10× speedup** while maintaining acceptable quality for many use cases.

## Current Model: SCUNet-GAN

| Property       | Value                                            |
| -------------- | ------------------------------------------------ |
| Architecture   | Swin Transformer + U-Net                         |
| Parameters     | ~15.2M                                           |
| Input Size     | Any (tiled to 256×256)                           |
| Output Scale   | 1× (same size)                                   |
| Purpose        | Denoising, artifact removal, general restoration |
| Inference Time | ~2.0s per 256×256 tile (CPU)                     |

---

## Alternative Models

### 1. Real-ESRGAN (Recommended)

**Real-ESRGAN-x4plus** is a popular super-resolution model that's faster than SCUNet.

| Property       | Value                                   |
| -------------- | --------------------------------------- |
| Architecture   | RRDB (Residual-in-Residual Dense Block) |
| Parameters     | ~16.7M                                  |
| Output Scale   | 4× upscaling                            |
| Inference Time | ~0.8s per 256×256 tile (CPU)            |
| **Speedup**    | **~2.5× faster**                        |

#### Variants

| Model                        | Size | Speed  | Quality | Best For               |
| ---------------------------- | ---- | ------ | ------- | ---------------------- |
| `RealESRGAN_x4plus`          | 64MB | Medium | High    | Photos, general images |
| `RealESRGAN_x4plus_anime_6B` | 17MB | Fast   | High    | Anime/illustrations    |
| `RealESRGAN_x2plus`          | 64MB | Medium | High    | 2× upscaling           |
| `realesr-general-x4v3`       | 64MB | Medium | Medium  | General purpose        |

#### Download & Convert

```python
# Download Real-ESRGAN model
import torch
from basicsr.archs.rrdbnet_arch import RRDBNet

# Load pretrained weights
model = RRDBNet(num_in_ch=3, num_out_ch=3, num_feat=64, num_block=23, num_grow_ch=32, scale=4)
model.load_state_dict(torch.load('RealESRGAN_x4plus.pth')['params_ema'])
model.eval()

# Export to ONNX
dummy_input = torch.randn(1, 3, 256, 256)
torch.onnx.export(
    model,
    dummy_input,
    "RealESRGAN_x4plus.onnx",
    opset_version=16,
    input_names=['input'],
    output_names=['output'],
    dynamic_axes={'input': {2: 'height', 3: 'width'}, 'output': {2: 'height', 3: 'width'}}
)
```

---

### 2. ESPCN (Very Fast)

**Efficient Sub-Pixel CNN** is an extremely lightweight model for real-time upscaling.

| Property       | Value                                 |
| -------------- | ------------------------------------- |
| Architecture   | Simple CNN with sub-pixel convolution |
| Parameters     | ~0.02M (20K)                          |
| Output Scale   | 2×, 3×, or 4×                         |
| Inference Time | ~0.01s per 256×256 tile (CPU)         |
| **Speedup**    | **~200× faster**                      |

#### Trade-off
- Much lower quality than SCUNet or Real-ESRGAN
- Best for real-time previews or video upscaling
- No denoising/artifact removal capability

#### Download

```python
import torch
import torch.nn as nn

class ESPCN(nn.Module):
    def __init__(self, scale_factor=4):
        super().__init__()
        self.conv1 = nn.Conv2d(3, 64, 5, padding=2)
        self.conv2 = nn.Conv2d(64, 32, 3, padding=1)
        self.conv3 = nn.Conv2d(32, 3 * scale_factor**2, 3, padding=1)
        self.pixel_shuffle = nn.PixelShuffle(scale_factor)

    def forward(self, x):
        x = torch.relu(self.conv1(x))
        x = torch.relu(self.conv2(x))
        x = self.pixel_shuffle(self.conv3(x))
        return x

# Export
model = ESPCN(scale_factor=4)
torch.onnx.export(model, torch.randn(1, 3, 256, 256), "espcn_x4.onnx", opset_version=16)
```

---

### 3. SwinIR (High Quality, Medium Speed)

**SwinIR** uses the same Swin Transformer architecture as SCUNet but is optimized differently.

| Property       | Value                          |
| -------------- | ------------------------------ |
| Architecture   | Swin Transformer               |
| Parameters     | ~11.8M (lightweight) to ~11.9M |
| Output Scale   | 2×, 3×, 4×, or 1× (denoising)  |
| Inference Time | ~1.2s per 256×256 tile (CPU)   |
| **Speedup**    | **~1.7× faster**               |

#### Variants

| Model                    | Task             | Parameters |
| ------------------------ | ---------------- | ---------- |
| `SwinIR_classical_sr_x4` | Classical SR     | 11.8M      |
| `SwinIR_real_sr_x4`      | Real-world SR    | 11.9M      |
| `SwinIR_denoise_15`      | Denoising (σ=15) | 11.8M      |
| `SwinIR_denoise_50`      | Denoising (σ=50) | 11.8M      |

#### Note on burn-import Compatibility
SwinIR also uses `Einsum` operations, so it's **not compatible** with burn-import for native Burn/Vulkan execution.

---

### 4. NAFNet (Balanced)

**NAFNet** (Nonlinear Activation Free Network) is a fast restoration model.

| Property       | Value                        |
| -------------- | ---------------------------- |
| Architecture   | U-Net with simplified blocks |
| Parameters     | ~17.1M                       |
| Output Scale   | 1× (restoration only)        |
| Inference Time | ~0.6s per 256×256 tile (CPU) |
| **Speedup**    | **~3.3× faster**             |

#### Key Feature
NAFNet uses no complex nonlinear activations (no softmax, GELU, etc.), making it very efficient.

```python
# Download from: https://github.com/megvii-research/NAFNet
# Model: NAFNet-REDS-width64.pth (for video deblurring)
# Model: NAFNet-GoPro-width64.pth (for motion deblurring)
```

---

### 5. Compact Models for Burn Compatibility

If you want to use **burn-import** for native Vulkan acceleration, you need models that avoid:
- `Einsum` (tensor contraction)
- `ScatterND` (advanced indexing)
- Complex attention mechanisms

#### Compatible Architectures

| Architecture                | Compatible | Notes                      |
| --------------------------- | ---------- | -------------------------- |
| ESPCN                       | ✅ Yes      | Simple CNN                 |
| SRCNN                       | ✅ Yes      | Original SR CNN            |
| FSRCNN                      | ✅ Yes      | Fast SRCNN                 |
| EDSR                        | ✅ Yes      | Enhanced Deep SR           |
| RDN                         | ✅ Yes      | Residual Dense Network     |
| RRDB (Real-ESRGAN backbone) | ⚠️ Maybe    | May need op simplification |
| SwinIR                      | ❌ No       | Uses Einsum                |
| SCUNet                      | ❌ No       | Uses Einsum, ScatterND     |
| NAFNet                      | ⚠️ Maybe    | Check specific ops         |

---

## Implementation: Model Selection

```rust
pub enum ModelType {
    /// High quality, slow (current default)
    SCUNetGAN,
    /// Fast 4× upscaling, good quality
    RealESRGAN,
    /// Very fast 4× upscaling, lower quality
    ESPCN,
    /// Balanced restoration
    NAFNet,
}

impl ModelType {
    pub fn model_path(&self) -> &'static str {
        match self {
            Self::SCUNetGAN => "models/SCUNet-GAN.onnx",
            Self::RealESRGAN => "models/RealESRGAN_x4plus.onnx",
            Self::ESPCN => "models/espcn_x4.onnx",
            Self::NAFNet => "models/NAFNet-width64.onnx",
        }
    }

    pub fn scale_factor(&self) -> u32 {
        match self {
            Self::SCUNetGAN | Self::NAFNet => 1,
            Self::RealESRGAN | Self::ESPCN => 4,
        }
    }

    pub fn expected_tile_time_ms(&self) -> u32 {
        match self {
            Self::SCUNetGAN => 2000,
            Self::RealESRGAN => 800,
            Self::NAFNet => 600,
            Self::ESPCN => 10,
        }
    }
}
```

---

## Model Conversion Script

```python
#!/usr/bin/env python3
"""Convert various SR models to ONNX format."""

import torch
import argparse

def convert_realesrgan(output_path: str, scale: int = 4):
    from basicsr.archs.rrdbnet_arch import RRDBNet

    model = RRDBNet(
        num_in_ch=3, num_out_ch=3,
        num_feat=64, num_block=23,
        num_grow_ch=32, scale=scale
    )

    # Download weights from:
    # https://github.com/xinntao/Real-ESRGAN/releases
    weights = torch.load(f'RealESRGAN_x{scale}plus.pth')
    model.load_state_dict(weights['params_ema'])
    model.eval()

    dummy = torch.randn(1, 3, 256, 256)
    torch.onnx.export(
        model, dummy, output_path,
        opset_version=16,
        input_names=['input'],
        output_names=['output'],
        dynamic_axes={
            'input': {0: 'batch', 2: 'height', 3: 'width'},
            'output': {0: 'batch', 2: 'height', 3: 'width'}
        }
    )
    print(f"Exported to {output_path}")

def convert_nafnet(output_path: str):
    # Clone: https://github.com/megvii-research/NAFNet
    from basicsr.models.archs.NAFNet_arch import NAFNet

    model = NAFNet(
        img_channel=3, width=64,
        middle_blk_num=12,
        enc_blk_nums=[2, 2, 4, 8],
        dec_blk_nums=[2, 2, 2, 2]
    )

    weights = torch.load('NAFNet-width64.pth')
    model.load_state_dict(weights['params'])
    model.eval()

    dummy = torch.randn(1, 3, 256, 256)
    torch.onnx.export(
        model, dummy, output_path,
        opset_version=16,
        input_names=['input'],
        output_names=['output']
    )
    print(f"Exported to {output_path}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", choices=["realesrgan", "nafnet"], required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--scale", type=int, default=4)
    args = parser.parse_args()

    if args.model == "realesrgan":
        convert_realesrgan(args.output, args.scale)
    elif args.model == "nafnet":
        convert_nafnet(args.output)
```

---

## Recommendation

| Use Case                   | Recommended Model    | Reason                              |
| -------------------------- | -------------------- | ----------------------------------- |
| **Quality-first**          | SCUNet-GAN (current) | Best denoising/restoration          |
| **Balanced speed/quality** | NAFNet               | 3× faster, good quality             |
| **Fast upscaling**         | Real-ESRGAN          | 2.5× faster, 4× upscale             |
| **Real-time preview**      | ESPCN                | 200× faster, acceptable for preview |
| **Burn/Vulkan native**     | ESPCN or EDSR        | Compatible with burn-import         |

---

## Next Steps

1. Download and convert Real-ESRGAN or NAFNet
2. Add model selection to `SuperResolutionBuilder`
3. Adjust tile handling for 4× upscaling models
4. Benchmark and compare quality/speed tradeoffs
