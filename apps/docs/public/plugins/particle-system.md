---
title: Particle system
order: 3
outline: deep
---

# Particle system

`ParticleSystem` scatters particles over an image: fire, smoke, sparks, snow, flares, fireworks. There is no loop; every value comes from the caller, and `progress` picks the moment of each particle's life that is drawn. The same seed gives the same picture.

The particle system is a library crate, `abra_particle_system`. It is not a runtime plugin (see [Plugins](./index)): it is Rust code for programs that use Abra on its own, such as a command line tool or a server. It makes the particles and hands them to the [sprite renderer](../core/drawing/sprites), which draws them on the GPU when there is one and on the CPU otherwise. An app can do the same by placing sprites itself.

```rust
use abra::prelude::*;
use abra_particle_system::prelude::*;
```

## Scatter particles

A particle system is an abra effect, so `apply` draws it (see [Apply options](../core/apply-options)):

```rust
let mut image = Image::read("input.png")?;

particle_system()
  .with_seed(7)
  .with_count(300)
  .with_emitter(Emitter::Circle { center: (400.0, 300.0), radius: 80.0 })
  .with_size((2.0, 8.0))
  .with_start_color(Gradient::from_to(Color::from_rgb(255, 200, 80), Color::from_rgb(255, 60, 20)))
  .with_start_speed((-30.0, 30.0), (-160.0, -60.0))
  .with_gravity(40.0)
  .with_blend(Blend::Additive)
  .with_progress(0.5)
  .apply(&mut image);
```

## Emitters

`with_emitter` says where particles are born, in pixels of the image:

| Emitter                                  | Where                                                        |
| ---------------------------------------- | ------------------------------------------------------------ |
| `Emitter::Image`                         | Anywhere on the image. The default.                          |
| `Emitter::Circle { center, radius }`     | Inside a circle.                                             |
| `Emitter::Ring { center, radius, inner_radius }` | On a ring between two radii.                         |
| `Emitter::Rect(left, top, width, height)` | Inside a rectangle.                                         |
| `Emitter::Line(from, to)`                | Along a line.                                                |
| `Emitter::Luminance(min)`                | On the parts of the image at least this light, `0.0` to `1.0`. |

## Values over a life

Most settings take a number, a `(min, max)` pair, a `Curve` or a pair of curves: a `CurveRange` (see [Random numbers, noise and ranges](../core/utilities#random-numbers-noise-and-ranges)). Each particle picks one place between the curves and keeps it, so no two are alike but each one still changes over its life.

| Function                                   | Sets                                                                                 |
| ------------------------------------------ | ------------------------------------------------------------------------------------ |
| `with_count`, `with_seed`, `with_progress` | How many, which scatter, and how far along their lives they are (`0.0` to `1.0`).    |
| `with_age_spread`                          | How far each particle's own age may differ, so they are not all the same age.        |
| `with_size`, `with_opacity`, `with_rotation` | Size in pixels, opacity and degrees over a life.                                   |
| `with_start_color`, `with_color_over_lifetime` | A gradient to pick a color from, and one the color follows over the life.        |
| `with_start_speed`, `with_velocity_over_lifetime` | Speed along x and y, in pixels per whole life.                                |
| `with_gravity`                             | A pull down the image. Negative pulls up.                                            |
| `with_orbital_velocity`, `with_radial_velocity`, `with_velocity_center` | Spin around a point and push away from it. |
| `with_noise(strength, swirl_size)`         | Smooth noise that pushes particles about. `with_noise_octaves`, `with_noise_scroll` and `with_noise_damping` add layers, slide it and make the strength a share of a swirl. |
| `with_launch(Impulse)`                     | Throws each particle at its own speed and direction, or straight out from the middle. |
| `with_drag`, `with_wind`                   | Lose the start speed over a life, and a constant push in x and y.                    |
| `with_bounds`                              | A box particles bounce around in, off its sides like walls.                          |
| `with_speed_response`, `with_pose_noise`   | Size, rotation and color that follow how fast a particle goes, and noise on size and rotation. |
| `with_velocity_stretch`                    | Longer along the way a particle goes, and turned that way.                           |
| `with_texture_frames`, `with_frame_cycles` | Treat the texture as a sheet of frames and play them over the life.                  |
| `with_emission(Schedule)`, `with_time`     | Particles born over time (a rate, bursts, a delay) that live for a lifetime picked from a range; the time shown is in seconds. |
| `with_hue_variation`                       | Turns the hue over the life.                                                         |
| `with_texture`, `with_blend`               | Draw an image instead of a dot, and how particles blend with the picture.            |
| `with_trail`, `with_trail_fade`, `with_trail_hardness` | A streak behind each particle.                                           |
| `with_sub_emitter(age, system)`            | Another system born from each particle once it reaches that age.                     |

With no color set, a particle takes the color of the pixel it was born on. Systems the shader cannot draw (a luminance emitter, a picture color that is turned in hue or put in a trail, a custom blend) are drawn on the CPU, with the same result.
