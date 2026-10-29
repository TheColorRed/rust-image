# GPU Acceleration Setup for Super-Resolution

## Prerequisites

Before enabling GPU support, you need:

1. **NVIDIA GPU** with CUDA support
2. **CUDA Toolkit** installed (TensorFlow 2.13 requires CUDA 11.8)
3. **cuDNN** installed (compatible with your CUDA version)

Check if you have these installed:
```bash
nvidia-smi           # Check GPU
nvcc --version       # Check CUDA
```

## Option 1: Use Pre-built GPU Binary (Simplest)

### Step 1: Clean the build cache
```bash
cargo clean
rm -rf target/
```

### Step 2: Update Cargo.toml
```toml
[dependencies]
tensorflow = { workspace = true, features = ["tensorflow_gpu"] }
```

### Step 3: Try building
```bash
cargo build -p super-resolution -j 1
```

**If you get "InvalidArchive" error**, the pre-built binary download is corrupted. Try:
- Delete the cached file: `target/release/build/tensorflow-sys-*/out/*.zip`
- Retry the build

## Option 2: Manual TensorFlow Compilation (Most Reliable)

If the pre-built binary doesn't work, compile TensorFlow from source with GPU support.

### Requirements:
- Bazel build system
- Python with NumPy
- SWIG
- CUDA and cuDNN installed

### Steps:

1. **Clone TensorFlow**:
```bash
git clone https://github.com/tensorflow/tensorflow
cd tensorflow
git checkout v2.13.0  # Match the version used by tensorflow-sys
```

2. **Configure for GPU**:
```bash
./configure
# Answer YES when asked about CUDA support
# Provide paths to CUDA toolkit and cuDNN
```

3. **Build TensorFlow shared library**:
```bash
bazel build --config=opt --config=cuda --jobs=1 //tensorflow:libtensorflow.so
# Using --jobs=1 is recommended unless you have lots of RAM
```

4. **Install the library**:
```bash
# Copy the built libraries
sudo cp bazel-bin/tensorflow/libtensorflow.so /usr/local/lib/
sudo cp bazel-bin/tensorflow/libtensorflow_framework.so /usr/local/lib/
sudo ldconfig

# Generate and install pkg-config file
./tensorflow/c/generate-pc.sh --prefix=/usr/local --version=2.13.0
sudo cp tensorflow.pc /usr/lib/pkgconfig/
```

5. **Verify installation**:
```bash
pkg-config --libs tensorflow
```

6. **Build super-resolution**:
```bash
cd /path/to/rust-image
cargo clean
cargo build -p super-resolution
```

## Option 3: Use System TensorFlow with GPU

If you have TensorFlow with GPU already installed on your system:

### On Windows:
Set environment variable before building:
```bash
set LIBTENSORFLOW=C:\path\to\tensorflow
cargo build -p super-resolution
```

### On Linux/Mac:
```bash
export LIBTENSORFLOW=/usr/local
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
cargo build -p super-resolution
```

## Verifying GPU Usage

Once built with GPU support, TensorFlow will automatically use the GPU if available. Check the logs:

```bash
cargo run -p blur-test
```

Look for messages like:
- `Created TensorFlow device (/job:localhost/replica:0/task:0/device:GPU:0 with...)`
- GPU memory allocation messages

## Performance Expectations

With GPU acceleration:
- **Inference time**: Should drop from ~60s to ~2-5s (10-30x faster)
- **Preprocessing/Postprocessing**: Already optimized with Rayon (~0.5s + ~0.2s)
- **Total time**: ~3-6s instead of ~60s

## Troubleshooting

### "libtensorflow.so: cannot open shared object file"
```bash
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
sudo ldconfig
```

### "InvalidArchive" error
The pre-built GPU binary download is corrupted. Delete cached files and retry, or use manual compilation.

### CUDA out of memory
The model requires significant GPU memory. If you get OOM errors:
- Close other GPU applications
- Reduce input image size
- Use a GPU with more VRAM

### GPU not being used
Check:
1. CUDA is properly installed: `nvidia-smi`
2. TensorFlow was built with GPU support
3. Look for GPU device creation messages in the logs
