# MediaPipe Selfie Multiclass model

`selfie_multiclass_256x256.onnx` matches `onnx/model.onnx` from
[`senty-au/selfie_multiclass_256x256-ONNX`](https://huggingface.co/senty-au/selfie_multiclass_256x256-ONNX):
the local file's SHA-256 is identical to the checksum published in that repository's model card.
This establishes a matching upstream artifact; the Git history does not record the original download URL.

The publisher converted Google's MediaPipe Selfie Multiclass model from TFLite to ONNX
using tf2onnx 1.17.0 with opset 17. The publisher lists the model and weights as copyright
Google LLC under Apache-2.0.

See the [Google MediaPipe image segmentation guide](https://developers.google.com/edge/mediapipe/solutions/vision/image_segmenter)
for the original model, and [`selfie_multiclass_256x256.yml`](selfie_multiclass_256x256.yml)
for source links, checksums, tensor layouts, and class labels. The YAML is the model's manifest: `BodySegmentation`
(and `AiModel`) read its `decoder` and `params` to feed the model and read its output.

SHA-256: `35ec1ecd9ee7f85073c99c00020b7f6751b69506eeacf683bc8665f6117f85b0`
