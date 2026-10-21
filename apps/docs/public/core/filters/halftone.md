---
title: Halftone
order: 8
outline: deep
---

# Halftone

`halftone` turns an image into a halftone screen, as a printed comic or newspaper is made: dots of one ink on a paper color, in rows turned by an angle, where each dot is as big as the image is dark under it. Light parts get small dots with paper between them. Dark parts get dots that grow until they touch and then join, leaving rounded gaps of paper that close into solid ink.

```rust
use abra::filters::prelude::halftone::halftone;

halftone(10.0).apply(&mut image);
```

The ink is black and the paper white unless you set them, so the result is black dots on white, as Photoshop's Halftone Pattern makes them. That can be multiplied over a flat color, which is how a pop art print puts dots over colored skin and hair.

## How big a dot is

The share of a dot's cell that is ink is the share that is dark. A cell that is half gray is half ink, and a cell that is white has no dot. The darkness of a cell is the mean of a grid of pixels spread over it (5 by 5 unless you set `with_samples`), so every pixel of a cell gets the same dot, and the size changes smoothly from one dot to the next as the picture does. Darkness is the brightness of a pixel (red, green and blue weighted by how bright each looks), times its alpha. A clear pixel counts as no darkness, and the alpha of the image is kept.

## Settings

```rust
use abra::abra_core::Color;

halftone(10.0)
  .with_angle(45.0) // the rows of dots, turned this many degrees; 0 is upright rows
  .with_ink(Color::from_rgb(225, 40, 50))
  .with_paper(Color::from_rgb(255, 214, 190))
  .with_samples(5) // how many pixels in each direction are looked at over a cell
  .apply(&mut image);
```

| Method                | Default | What it does                                                              |
| --------------------- | ------- | ------------------------------------------------------------------------- |
| `halftone(spacing)`   |         | The distance in pixels between the dots of a row. Less than 1 is taken as 1. |
| `with_angle(degrees)` | 45      | How far the rows of dots are turned. A printed screen is turned 45 degrees. |
| `with_ink(color)`     | black   | The color of the dots. Its alpha is not used.                             |
| `with_paper(color)`   | white   | The color between the dots. Its alpha is not used.                        |
| `with_samples(n)`     | 5       | How many pixels in each direction are looked at over a cell. More follows fine detail better and is slower. |

## Where the dots start

The rows of dots start from the corner of the image, so the dots do not move when the effect is limited to an [area or mask](../apply-options). It runs on the GPU as well as the CPU, and the two make the same screen.

## Tone first

The dots follow the brightness of the image as it is, so even out its tones first for a graphic result. Applying `auto_tone` to a grayscale copy makes the darkest parts solid ink and the lightest bare paper, and a copy of the image given a halftone can be laid over a photo or a flat color with a blend mode such as Multiply.
