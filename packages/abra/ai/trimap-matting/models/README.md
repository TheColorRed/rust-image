# ViTMatte models

Two interchangeable models, both the quantized ONNX export by Xenova of the `hustvl` ViTMatte weights trained on
Composition-1k (license Apache-2.0 on the `hustvl` small model's card; the Xenova base page states none, so confirm it on
`hustvl/vitmatte-base-composition-1k` before shipping the base model):

| File | Source | Size | SHA-256 |
|---|---|---|---|
| `vitmatte-small-composition-1k.onnx` | [`Xenova/vitmatte-small-composition-1k`](https://huggingface.co/Xenova/vitmatte-small-composition-1k) `onnx/model_quantized.onnx` | 27.5 MB | `8216103d998864e122333e17d7d346feb1d6558d96bf08695a9838dba4f114e0` |
| `vitmatte-base-composition-1k.onnx` | [`Xenova/vitmatte-base-composition-1k`](https://huggingface.co/Xenova/vitmatte-base-composition-1k) `onnx/model_quantized.onnx` | 99 MB | `fafacae424df98b6c849b269d8973929419765058863da5e6500290ac2f50406` |

Both take `pixel_values` `[1, 4, height, width]` (the photo, then the trimap) and give `alphas` `[1, 1, height, width]`.
Each `.yml` is the model's manifest, read from beside the `.onnx` by `TrimapMatting::load`. They are the same apart from
the name and checksum, so switching model is passing another path. A model file with a manifest of its own, such as the
full-precision `model.onnx` of either size, loads the same way.

## Quality

Checked on a synthetic photo with a trimap (see the tests in `src/lib.rs`):

- **small, quantized**: correct. Sure subject stays 255, sure background stays 0, and the unknown band flips at the edge.
- **base, full precision (387 MB, not kept in the repo)**: correct, the same as the small model.
- **base, quantized (the 99 MB file here)**: noisy. It gave 30 to 60 on pixels the trimap says are sure subject. The input
  is fed correctly (the full-precision model of the same size is clean), so this is the int8 quantization. Test it on real
  photos before relying on it; the small quantized model is the better choice until then.
