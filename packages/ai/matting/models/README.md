# Robust Video Matting model

`rvm_mobilenetv3_fp32.onnx` is `rvm_mobilenetv3_fp32.onnx` from the v1.0.0 release of
[`PeterL1n/RobustVideoMatting`](https://github.com/PeterL1n/RobustVideoMatting/releases/tag/v1.0.0)
(downloaded from `https://github.com/PeterL1n/RobustVideoMatting/releases/download/v1.0.0/rvm_mobilenetv3_fp32.onnx`).

**License: GPL-3.0** (the repository's license). The other models in Abra are Apache-2.0 or similar, so check this before
publishing the library or shipping an app with this model.

Inputs and outputs, read from the file: `src` (RGB 0..1), the four recurrent states `r1i`..`r4i`, `downsample_ratio`;
outputs `fgr`, `pha` (the alpha) and `r1o`..`r4o`. [`rvm_mobilenetv3_fp32.yml`](rvm_mobilenetv3_fp32.yml) is the manifest
with the settings `Matting` feeds it. The model has recurrent states for video; a single photo starts from empty ones.

SHA-256: `88d4531297118f595bf2fd60f6f566aec2e559393802d1f436c380f0cbbd2828`
