---
title: Body parts and models on a region
order: 20
outline: deep
---

# Body parts, and running a model on a region

## Body parts

`BodyParser` labels the regions of a person in a picture: the background, the hair, the skin of the body, the skin of the face, the clothes and the accessories. It takes the same `.onnx` file as `BodySegmentation`, which only gives the skin as one mask. The library never looks for models, so you give the path.

```rust
use abra_body_segmentation::prelude::*;

let parser = BodyParser::load("models/selfie_multiclass_256x256.onnx")?;
let parts = parser.process(&photo)?;

let hair = parts.mask(BodyRegion::Hair);
let skin = parts.mask_of(&[BodyRegion::BodySkin, BodyRegion::FaceSkin]);
let region = parts.region_at(120, 80); // Some(BodyRegion::FaceSkin), say
```

Each mask is hard (`255` or `0`) and the size of the picture. A model sees a small copy of the picture, so the edges are rough; a guided filter (see [Masks](../mask/masks)) makes a mask follow the edges of the photo.

| `BodyRegion`  | What it is                                             |
| ------------- | ------------------------------------------------------ |
| `Background`  | Everything that is not the person.                     |
| `Hair`        | Hair.                                                  |
| `BodySkin`    | Skin of the body, not the face.                        |
| `FaceSkin`    | Skin of the face.                                      |
| `Clothes`     | Clothes.                                               |
| `Accessories` | Glasses, jewelry and other things worn that are not clothes. |

## Run a model on a region

Any model that gives a mask or labels sees a small copy of what it is given. A person who is a small part of a photo is only a few pixels to it, and the result is rough or has holes. `process_within` gives it just the part that matters:

```rust
use abra_core::Rect;

// The bounds of the person, from a matting model, with a tenth more on each side.
let bounds = matte.bounds(128).unwrap();
let parts = parser.process_within(&photo, bounds, 0.1)?;
```

The part is cut out of the picture, processed, and the result is put back where the part was, so it is the size of the whole picture as if the model had been run on all of it. Pixels outside the part are `0` in a mask and have no class in labels (`LabelMap::NO_CLASS`, and no region for `BodyParser` and `FaceParser`).

`process_within` is on every `ImageAi` model whose output is `Placeable`: `Mask`, `LabelMap`, `BodySegments` and `FaceSegmentation`. The margin is a share of the larger side of the region, and the part is kept inside the picture. A region outside the picture is an error.
