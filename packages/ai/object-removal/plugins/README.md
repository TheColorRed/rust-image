# Object removal plugins

The folders here start with `ai-` because they add AI models; that is a convention, and a plugin is named by the `id` in its `plugin.yml`, not by its folder.

Each folder here is a runtime plugin (see `packages/react-native/alakazam/src/plugins`): a `plugin.yml` that says what it adds, and the files it names. Each adds one AI
model with the `inpaint` capability. The model has a `.yml` manifest beside it that says how it is fed (see `abra_ai_core::graph`).

## MI-GAN (`migan-object-removal`)

`migan_pipeline_v2.onnx` is MI-GAN (Picsart AI Research, ICCV 2023), exported as a pipeline by
[`andraniksargsyan/migan`](https://github.com/andraniksargsyan/migan) and mirrored, unchanged, at
[`CardAnvilModels/MI-GAN-ONNX`](https://huggingface.co/CardAnvilModels/MI-GAN-ONNX)
(`https://huggingface.co/CardAnvilModels/MI-GAN-ONNX/resolve/main/migan_pipeline_v2.onnx`).

**License: MIT.** 28 MB.

Inputs and outputs, read from the file: `image` (`uint8`, `[batch, 3, height, width]`, RGB), `mask` (`uint8`,
`[batch, 1, height, width]`, `0` on the part to fill: checked by the crate's tests) and `result` (`uint8`,
`[batch, 3, height, width]`). The pipeline crops around the masked area, fills it at 512 x 512 and pastes the result back, so the
output is the size of the input. [`migan_pipeline_v2.yml`](migan_pipeline_v2.yml) is its manifest.

SHA-256: `6f1f3530a1a2324b19752018ce756088b07973cda8d7d890034ace5c8a48c40b`

## LaMa (`lama-object-removal`)

`lama_fp32.onnx` is big-lama (Samsung AI Center, WACV 2022), exported to ONNX by
[`Carve-Photos/lama`](https://github.com/Carve-Photos/lama) and published at
[`Carve/LaMa-ONNX`](https://huggingface.co/Carve/LaMa-ONNX)
(`https://huggingface.co/Carve/LaMa-ONNX/resolve/main/lama_fp32.onnx`).

**License: Apache-2.0.** **208 MB, so it is not in git** (`.gitignore` leaves it out). Download it into `ai-lama-object-removal/` to use the plugin. Without it the
plugin is reported as having a missing file, and the other plugins still work.

Inputs and outputs, read from the file: `image` (`float32`, `[batch, 3, 512, 512]`, RGB from 0 to 1), `mask` (`float32`,
`[batch, 1, 512, 512]`, `1` on the hole) and `output` (`float32`, `[batch, 3, 512, 512]`, from 0 to 255). The size is fixed, so
`ObjectRemoval` cuts a square round the mask, fills it and pastes only the filled part back. [`lama_fp32.yml`](lama_fp32.yml) is its
manifest.

SHA-256: `1faef5301d78db7dda502fe59966957ec4b79dd64e16f03ed96913c7a4eb68d6`
