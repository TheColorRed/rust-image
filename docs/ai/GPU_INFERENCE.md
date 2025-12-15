# GPU Inference for Super-Resolution

## Overview

GPU inference would provide **10-50× speedup** over CPU for neural network models like SCUNet-GAN. Currently, the super-resolution pipeline runs on CPU taking ~80 seconds for a 1087×1087 image. With GPU acceleration, this could drop to **2-8 seconds**.

## Current Status

| Backend           | Status          | Notes                                                                   |
| ----------------- | --------------- | ----------------------------------------------------------------------- |
| **DirectML**      | ❌ Crashes       | `STATUS_ACCESS_VIOLATION` on AMD GPUs during inference                  |
| **WebGPU**        | ❌ Linker issues | Windows `kernel32.dll` import library conflicts with `dylib` crate type |
| **CUDA**          | ⚠️ Not tested    | Requires NVIDIA GPU                                                     |
| **ROCm/MIGraphX** | ⚠️ Linux only    | AMD's native GPU compute stack                                          |
| **burn-import**   | ❌ Incompatible  | SCUNet uses unsupported ops (`Einsum`, `ScatterND`)                     |

---

## Option 1: Fix DirectML (Windows, AMD/NVIDIA/Intel)

### What is DirectML?
DirectML is Microsoft's hardware-accelerated machine learning API built on DirectX 12. It supports AMD, NVIDIA, and Intel GPUs on Windows.

### Current Issue
```
STATUS_ACCESS_VIOLATION during model inference
- Occurs after successful session creation
- Happens on AMD Radeon GPUs
- May be driver or ONNX Runtime compatibility issue
```

### Investigation Steps

1. **Update GPU drivers to latest version**
   ```powershell
   # Check current driver version
   wmic path win32_VideoController get name,driverversion
   ```

2. **Test with different ONNX Runtime versions**
   ```toml
   # Try older stable version
   ort = { version = "1.16", features = ["directml"] }
   ```

3. **Simplify the model for testing**
   ```python
   # Export a minimal test model
   import torch
   import torch.nn as nn

   class SimpleConv(nn.Module):
       def __init__(self):
           super().__init__()
           self.conv = nn.Conv2d(3, 3, 3, padding=1)

       def forward(self, x):
           return self.conv(x)

   model = SimpleConv()
   dummy = torch.randn(1, 3, 256, 256)
   torch.onnx.export(model, dummy, "simple_conv.onnx", opset_version=16)
   ```

4. **Enable DirectML debug layer**
   ```rust
   use ort::execution_providers::DirectMLExecutionProvider;

   let dml = DirectMLExecutionProvider::default()
       .with_device_id(0)
       .with_debug_layer(true);  // Enable debug output
   ```

5. **Check for operator support issues**
   ```bash
   # List operators in the model
   python -c "import onnx; m = onnx.load('SCUNet-GAN.onnx'); print(set(n.op_type for n in m.graph.node))"
   ```

### Implementation (once working)

```rust
use ort::execution_providers::{DirectMLExecutionProvider, ExecutionProvider};

fn try_directml_session(model_bytes: &[u8]) -> Option<Session> {
    let dml = DirectMLExecutionProvider::default();

    if !dml.is_available().unwrap_or(false) {
        eprintln!("DirectML not available");
        return None;
    }

    Session::builder()
        .ok()?
        .with_execution_providers([dml.build()])
        .ok()?
        .commit_from_memory(model_bytes)
        .ok()
}
```

---

## Option 2: Fix WebGPU/Vulkan (Cross-platform)

### What is WebGPU?
WebGPU is a modern graphics/compute API that abstracts over Vulkan (Windows/Linux), Metal (macOS), and DX12 (Windows). ONNX Runtime's WebGPU EP uses this for GPU inference.

### Current Issue
```
error: error creating import library for kernel32.dll: The file exists. (os error 80)
```

This occurs when linking the `sr-test` binary due to conflicts between:
- `wgpu-core-deps-windows-linux-android` crate
- Multiple crates creating the same Windows import libraries

### Potential Fixes

1. **Change crate-type from `dylib` to `rlib`** ✅ Already done
   ```toml
   [lib]
   crate-type = ["rlib"]  # Instead of ["dylib"]
   ```

2. **Use `--target-dir` to isolate builds**
   ```bash
   cargo build --package sr-test --target-dir target/sr-test
   ```

3. **Build in release mode** (different linking behavior)
   ```bash
   cargo build --package sr-test --release
   ```

4. **Disable parallel codegen units**
   ```toml
   # Cargo.toml
   [profile.dev]
   codegen-units = 1
   ```

### Implementation (once working)

```rust
use ort::execution_providers::WebGPUExecutionProvider;

fn try_webgpu_session(model_bytes: &[u8]) -> Option<Session> {
    let webgpu = WebGPUExecutionProvider::default();

    if !webgpu.is_available().unwrap_or(false) {
        eprintln!("WebGPU not available");
        return None;
    }

    Session::builder()
        .ok()?
        .with_execution_providers([webgpu.build()])
        .ok()?
        .commit_from_memory(model_bytes)
        .ok()
}
```

---

## Option 3: Use CUDA (NVIDIA GPUs)

### Requirements
- NVIDIA GPU (GTX 10xx or newer recommended)
- CUDA Toolkit 11.x or 12.x
- cuDNN library

### Setup

1. **Install CUDA Toolkit**
   - Download from: https://developer.nvidia.com/cuda-downloads
   - Add to PATH: `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.x\bin`

2. **Install cuDNN**
   - Download from: https://developer.nvidia.com/cudnn
   - Copy DLLs to CUDA bin directory

3. **Enable CUDA in ort**
   ```toml
   ort = { version = "2.0.0-rc.10", features = ["cuda"] }
   ```

### Implementation

```rust
use ort::execution_providers::CUDAExecutionProvider;

fn try_cuda_session(model_bytes: &[u8]) -> Option<Session> {
    let cuda = CUDAExecutionProvider::default()
        .with_device_id(0)
        .with_arena_extend_strategy(ArenaExtendStrategy::SameAsRequested);

    if !cuda.is_available().unwrap_or(false) {
        eprintln!("CUDA not available");
        return None;
    }

    Session::builder()
        .ok()?
        .with_execution_providers([cuda.build()])
        .ok()?
        .commit_from_memory(model_bytes)
        .ok()
}
```

---

## Option 4: Use ROCm/MIGraphX (AMD GPUs, Linux)

### Requirements
- AMD GPU (RX 5000 series or newer)
- Linux operating system
- ROCm 5.x or 6.x installed

### Setup (Ubuntu/Debian)

```bash
# Add ROCm repository
wget https://repo.radeon.com/rocm/rocm.gpg.key -O - | sudo apt-key add -
echo 'deb [arch=amd64] https://repo.radeon.com/rocm/apt/6.0 ubuntu main' | sudo tee /etc/apt/sources.list.d/rocm.list

# Install ROCm
sudo apt update
sudo apt install rocm-hip-sdk migraphx

# Add user to video group
sudo usermod -aG video $USER
```

### Implementation

```rust
use ort::execution_providers::MIGraphXExecutionProvider;

fn try_migraphx_session(model_bytes: &[u8]) -> Option<Session> {
    let migraphx = MIGraphXExecutionProvider::default()
        .with_device_id(0)
        .with_fp16(true);  // Use FP16 for 2x memory savings

    if !migraphx.is_available().unwrap_or(false) {
        eprintln!("MIGraphX not available (requires ROCm on Linux)");
        return None;
    }

    Session::builder()
        .ok()?
        .with_execution_providers([migraphx.build()])
        .ok()?
        .commit_from_memory(model_bytes)
        .ok()
}
```

---

## Option 5: Use Burn with Vulkan Backend

### What is Burn?
Burn is a pure-Rust deep learning framework with native Vulkan support via `wgpu`.

### Limitation
**burn-import** cannot convert SCUNet-GAN because it uses unsupported ONNX operators:
- `Einsum` - Complex tensor contractions (Swin Transformer attention)
- `ScatterND` - Advanced scatter/indexing operations

### Alternative: Use a Burn-Compatible Model
If we switch to a simpler model architecture (see `SMALLER_MODELS.md`), burn-import might work:

```toml
[build-dependencies]
burn-import = "0.19"
```

```rust
// build.rs
use burn_import::onnx::ModelGen;

fn main() {
    ModelGen::new()
        .input("models/simple_model.onnx")
        .out_dir("src/model/")
        .run_from_onnx();
}
```

---

## Recommended Priority

1. **Try CUDA first** (if you have access to an NVIDIA GPU for testing)
2. **Debug DirectML** (most likely to work on Windows with AMD)
3. **Test WebGPU in release mode** (cross-platform potential)
4. **Consider Linux + ROCm** for AMD GPU acceleration

---

## Expected Performance Gains

| Backend       | Tiles/sec (est.) | Total Time (36 tiles) |
| ------------- | ---------------- | --------------------- |
| CPU (current) | 0.45             | ~80 seconds           |
| DirectML      | 5-10             | 4-8 seconds           |
| CUDA          | 10-20            | 2-4 seconds           |
| WebGPU/Vulkan | 3-8              | 5-12 seconds          |
| ROCm/MIGraphX | 8-15             | 3-5 seconds           |

*Estimates based on typical GPU vs CPU performance ratios for transformer models.*
