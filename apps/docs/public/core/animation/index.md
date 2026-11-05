---
title: Animation
order: 0
outline: deep
---

# Animation

The `animation` crate works out where things are, and how they look, at a moment of their life. It works out numbers for thousands of things at once, in one call, and it knows nothing about pictures or what the things are. What draws them lives in the crates that draw: for sprites and trails, that is [`Sprites`](../drawing/sprites).

It needs nothing but the library, so it works headless, in a CLI, on a server or across a language binding, where one call for thousands of things is far cheaper than a call for each.

An age is a number from 0 (the start of a thing's life) to 1 (the end). A curve's `x` runs along that life from 0 to 255, as for [`CurveRange`](../utilities).

## Motion

`Motion` says how things move over a life: a velocity that follows a curve, turning around a point (`orbital`), moving away from it (`radial`), gravity, wind, drag on the start speed, and smooth noise (with layers, a slide across the picture, and a damping that makes the strength a share of one swirl). `with_bounds(left, top, right, bottom)` makes things bounce off the sides of a box like walls, without losing speed. `velocity_at` and `speed_at` say which way and how fast a thing is going. `Mover` is one thing that moves: where it starts, the point it turns around, its start speed and what it picked at random. `motion.at(&mover, age)` is where it is, and `motion.at_many(&movers, &which, &ages)` is where many are in one call. A mover can be listed more than once at different ages, which is how a path is worked out.

```rust
let motion = Motion::new().with_gravity(30.0).with_noise((0.0, 12.0), 8.0, 7);
let (x, y) = motion.at(&mover, 0.5);
```

## Impulse and Schedule

`Impulse` throws things when they are born: a speed picked from a range, in a direction within a spread either side of an angle (0 is up, 90 is right), or straight out from the point they move around. `impulse.velocity_many(..)` does many in one call.

`Schedule` says when things are born and how long each lives, over time in seconds: a steady `rate` a second, `bursts` of many at once, a `delay`, and a lifetime picked from a range. It can run once or go round and round. Things are numbered, bursts first, and `schedule.age(index, time, pick)` says how far through its life a thing is, or that it is not alive then.

## Ensemble

An `Ensemble` is many things that live and end together. It holds where each starts, what it picked at random, and the ranges (size, opacity, rotation, hue) and the color its life follows. `ensemble.evaluate(progress)` says how every one looks at a moment, as lists of numbers, with the members of one ensemble worked out on all the cores:

- a **pose** for each member: `x`, `y`, `size`, `opacity`, `red`, `green`, `blue`, `rotation`, where it started, how much longer than wide it is (`stretch`) and which `frame` it is on;
- a **history** for each member that has moved: the pose at earlier ages along its path (a trail is drawn from it).

The look of a member can follow how fast it goes (`SpeedResponse`: a size multiplier and a rotation as curves, and a color table, over a range of speeds), be changed by noise where it is (`PoseNoise`: size and rotation), be stretched along the way it goes (`with_velocity_stretch`, which also turns it that way), and step through the frames of a sheet over its life (`with_frames`).

Members can be given a `Schedule` so they last over time (`evaluate` then takes seconds), and other ensembles can start from them once they are old enough (`with_child`). Each ensemble has a `tag`, a whole number that comes back on every run it makes, so whoever draws can tell where each came from.

The particle system uses it: its `Swarm` turns an `Ensemble`'s poses and histories into sprites and trails.

## Related

- [Sprites and trails](../drawing/sprites)
- [Particle system](../../plugins/particle-system)
