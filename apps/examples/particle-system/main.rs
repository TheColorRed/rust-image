use abra::adjustments::prelude::*;
use abra::mask::prelude::*;
use abra::abra_core::blend::blend;
use abra::prelude::*;
use abra_particle_system::prelude::*;

pub fn main() {
  // The app owns every value here; the plugin only reads what it is given.
  let image = Image::read("assets/kelsey.jpg").unwrap();

  let started = std::time::Instant::now();
  let burning = fire(&image);
  println!("fire took {:?}", started.elapsed());
  burning.write("assets/out/particle_system_result.png", None).unwrap();

  let started = std::time::Instant::now();
  let flaring = flares(&image);
  println!("flares took {:?}", started.elapsed());
  flaring.write("assets/out/particle_system_flares.png", None).unwrap();

  let started = std::time::Instant::now();
  let winged = wings(&image);
  println!("wings took {:?}", started.elapsed());
  winged.write("assets/out/particle_system_wings.png", None).unwrap();

  let started = std::time::Instant::now();
  let splashed = splash(&image);
  println!("splash took {:?}", started.elapsed());
  splashed.write("assets/out/particle_system_splash.png", None).unwrap();

  let started = std::time::Instant::now();
  let fireworks = fireworks(&image);
  println!("fireworks took {:?}", started.elapsed());
  fireworks.write("assets/out/particle_system_fireworks.png", None).unwrap();
}

/// Fire rising from the bottom of the picture, with smoke born from the flames.
fn fire(p_image: &Image) -> Image {
  let (width, height) = p_image.dimensions::<u32>();
  let (width, height) = (width as f32, height as f32);

  // Smoke: born from each flame particle near the end of its life, drifting up, growing and fading.
  let smoke = particle_system()
    .with_count(1)
    .with_seed(11)
    .with_size((Curve::from_to(14.0, 30.0), Curve::from_to(22.0, 48.0)))
    .with_start_color(Gradient::evenly(vec![
      Color::from_hex(0x3a3a3a),
      Color::from_hex(0x5a5a5a),
      Color::from_hex(0x7a7a7a),
    ]))
    .with_opacity((Curve::from_to(0.10, 0.0), Curve::from_to(0.18, 0.02)))
    .with_progress(0.6)
    .with_age_spread(0.4)
    .with_start_speed((-60.0, 60.0), (-300.0, -120.0))
    .with_velocity_over_lifetime(Curve::cosine(280.0, 1.0), 0.0);

  // A single flame: born in a small spot, rising in a narrow tongue; white-yellow at the base, red at the tip.
  particle_system()
    .with_emitter(Emitter::Circle {
      center: (width * 0.5, height * 0.92),
      radius: width * 0.04,
    })
    .with_seed(3)
    .with_count(3000)
    .with_size((Curve::from_to(6.0, 1.0), Curve::from_to(12.0, 3.0)))
    .with_color_over_lifetime(Gradient::evenly(vec![
      Color::from_hex(0xfff4c0),
      Color::from_hex(0xffc13a),
      Color::from_hex(0xff6a10),
      Color::from_hex(0xb81c05),
      Color::from_hex(0x30080a),
    ]))
    .with_blend(Blend::Additive)
    .with_opacity((Curve::from_to(0.35, 0.0), Curve::from_to(0.6, 0.05)))
    .with_progress(0.5)
    .with_age_spread(0.5)
    .with_start_speed((-8.0, 8.0), (-560.0, -300.0))
    .with_velocity_over_lifetime(Curve::cosine(380.0, 5.0), 0.0)
    .with_sub_emitter(0.8, smoke)
    .applied(p_image)
}

/// Flares like an aircraft drops: two bursts leave the shoulders, arc outward and fall. The long pale streaks are the
/// smoke each flare leaves behind (its trail), and the light is only the glowing head at the end of it. `p_side` is -1.0
/// for a burst that goes left and 1.0 for one that goes right.
fn burst(p_origin: (f32, f32), p_side: f32, p_seed: u64) -> [ParticleSystem; 2] {
  // The trail and the head must move exactly alike, so a head lands at the end of its own trail: the same seed, count,
  // emitter, speeds, gravity, noise and age, and nothing that draws a different number of random values.
  let flare = || {
    particle_system()
      .with_seed(p_seed)
      .with_emitter(Emitter::Circle {
        center: p_origin,
        radius: 14.0,
      })
      .with_count(260)
      .with_start_speed((40.0 * p_side, 300.0 * p_side), (-640.0, -300.0))
      .with_gravity(1100.0)
      .with_noise(22.0, 120.0)
      .with_progress(0.88)
      .with_age_spread(0.12)
  };
  // Smoke: thin dark ribbons that fade toward the tail, painted over the picture.
  let smoke = flare()
    .with_size((Curve::from_to(1.2, 3.0), Curve::from_to(1.8, 4.5)))
    .with_trail(0.6)
    .with_opacity(0.5)
    .with_trail_hardness(0.8)
    .with_color_over_lifetime(Gradient::evenly(vec![
      Color::from_hex(0x6b7686),
      Color::from_hex(0x3e4553),
      Color::from_hex(0x1c2029),
    ]));
  // Light: a glowing head on some of the flares, brighter on some than others.
  let heads = flare()
    .with_size((Curve::from_to(1.4, 2.0), Curve::from_to(2.4, 3.4)))
    .with_opacity((0.0, 1.0))
    .with_color_over_lifetime(Gradient::evenly(vec![
      Color::from_hex(0xfff2d0),
      Color::from_hex(0xffb35a),
      Color::from_hex(0xff7a20),
    ]))
    .with_blend(Blend::Additive);
  [smoke, heads]
}

fn flares(p_image: &Image) -> Image {
  let (width, height) = p_image.dimensions::<u32>();
  let (width, height) = (width as f32, height as f32);
  let shoulders = [
    ((width * 0.52, height * 0.235), -1.0, 100),
    ((width * 0.69, height * 0.245), 1.0, 200),
  ];
  let mut image = p_image.clone();
  for (origin, side, seed) in shoulders {
    for system in burst(origin, side, seed) {
      system.apply(&mut image);
    }
  }
  image
}

/// A smoke photo (white on black) made into a white texture whose opacity is how bright the smoke is. Everything darker than `p_black`
/// is clear, which gets rid of the grey the dark parts of a photo have, `p_gamma` below 1 thins the smoke out, and a vignette fades it
/// to nothing from `p_fade_from` (a share of the way to the edge) so that a stamp of it has no visible border.
fn smoke_texture(p_path: &str, p_black: f32, p_white: f32, p_gamma: f32, p_fade_from: f32) -> Image {
  let mut photo = Image::read(p_path).unwrap();
  levels::levels(p_black, p_white).with_gamma(p_gamma).apply(&mut photo);
  let (width, height) = photo.dimensions::<u32>();
  let mut texture = Image::new_from_color(width, height, Color::white());
  Mask::from_image(photo).apply_to_image(&mut texture);
  let fade = color::vignette(width, height, Color::black(), p_fade_from, 1.0);
  let keep: Vec<u8> = fade.rgba().chunks_exact(4).map(|pixel| 255 - pixel[3]).collect();
  Mask::from_values(width, height, keep).multiply_alpha(&mut texture);
  texture
}

/// Dark feathered wings rising out of blue smoke. Each feather is a long tapered stroke, wide where it leaves the shoulder and
/// pointed at the tip, and the smoke is big soft puffs and thin pale wisps pushed about by noise.
fn wings(p_image: &Image) -> Image {
  let (width, height) = p_image.dimensions::<u32>();
  let (width, height) = (width as f32, height as f32);

  // Smoke puffs: soft and big, many of them, each faint, painted over the picture.
  let puffs = |rect: (f32, f32, f32, f32), seed: u64, count: usize, colors: (u32, u32), opacity: (f32, f32)| {
    particle_system()
      .with_seed(seed)
      .with_emitter(Emitter::Rect(rect.0, rect.1, rect.2, rect.3))
      .with_count(count)
      .with_size((70.0, 150.0))
      .with_opacity(opacity)
      .with_start_color(Gradient::from_to(Color::from_hex(colors.0), Color::from_hex(colors.1)))
      .with_noise(140.0, 160.0)
      .with_start_speed((-60.0, 60.0), (-90.0, 30.0))
      .with_progress(0.6)
      .with_age_spread(0.4)
  };
  // A row of feathers along the front edge of a wing, from `p_from` (the shoulder) to `p_to` (the wrist), each going up and out to
  // one side. `p_reach` is how far out and up they go, in pixels.
  let feathers = |from: (f32, f32), to: (f32, f32), side: f32, seed: u64, count: usize, reach: ((f32, f32), (f32, f32)), thick: (f32, f32)| {
    particle_system()
      .with_seed(seed)
      .with_emitter(Emitter::Line(from, to))
      .with_count(count)
      .with_size((Curve::from_to(thick.0, 1.0), Curve::from_to(thick.1, 2.0)))
      .with_opacity(0.97)
      .with_color_over_lifetime(Gradient::evenly(vec![
        Color::from_hex(0x03060b),
        Color::from_hex(0x08192a),
        Color::from_hex(0x1e4d69),
      ]))
      .with_start_speed((reach.0.0 * side, reach.0.1 * side), reach.1)
      .with_gravity(260.0)
      .with_noise(14.0, 90.0)
      .with_progress(1.0)
      .with_trail(1.0)
      .with_trail_fade(0.0)
      .with_trail_hardness(0.9)
  };

  // Each wing's front edge runs from the shoulder up and out to the wrist.
  let (left_wing, right_wing) = (((width * 0.5, height * 0.24), (width * 0.25, height * 0.2)), ((width * 0.69, height * 0.25), (width * 0.94, height * 0.2)));
  let mut image = p_image.clone();
  // Behind the wings: dark teal smoke down the sides, and a pale glow beside her head, kept off her.
  for system in [
    puffs((0.0, 0.0, width * 0.42, height), 1, 520, (0x082f48, 0x15607f), (0.05, 0.11)),
    puffs((width * 0.74, 0.0, width * 0.26, height), 2, 400, (0x082f48, 0x15607f), (0.05, 0.11)),
    puffs((width * 0.18, height * 0.05, width * 0.26, height * 0.3), 3, 120, (0x9fd0e6, 0xd6f0fa), (0.015, 0.04)),
    puffs((width * 0.76, height * 0.05, width * 0.24, height * 0.3), 4, 120, (0x9fd0e6, 0xd6f0fa), (0.015, 0.04)),
  ] {
    system.apply(&mut image);
  }
  // Smoke photos, white on black, made into textures whose opacity is how bright the smoke is. Painted over the picture they darken
  // it, and added to it they light it; the cloud bank is the body of both, and the wisps and curls are the fine detail.
  let cloud = smoke_texture("assets/smoke/smoke-cloud-bank.png", 25.0, 200.0, 0.8, 0.7);
  let wisp = smoke_texture("assets/smoke/smoke-wisps.png", 35.0, 220.0, 0.9, 0.45);
  let curl = smoke_texture("assets/smoke/smoke-curls.png", 35.0, 220.0, 0.9, 0.45);
  let smoke = |texture: &Image, rect: (f32, f32, f32, f32), seed: u64, count: usize, size: (f32, f32), opacity: (f32, f32), tint: (u32, u32), blend: Blend| {
    particle_system()
      .with_seed(seed)
      .with_emitter(Emitter::Rect(rect.0, rect.1, rect.2, rect.3))
      .with_count(count)
      .with_size(size)
      .with_opacity(opacity)
      .with_rotation((-25.0, 25.0))
      .with_start_color(Gradient::from_to(Color::from_hex(tint.0), Color::from_hex(tint.1)))
      .with_texture(texture.clone())
      .with_blend(blend)
      .with_progress(0.5)
  };
  let (normal, add) = (Blend::default(), Blend::Additive);
  let (dark, deep) = ((0x03121c, 0x0b3a56), (0x020b12, 0x082a40));
  // Behind the wings: dark teal over the top and the corners, then a pale glow up the middle where the wings spread from.
  for system in [
    smoke(&cloud, (0.0, 0.0, width, height * 0.3), 44, 10, (420.0, 620.0), (0.55, 0.85), dark, normal),
    smoke(&cloud, (0.0, 0.0, width * 0.3, height), 45, 9, (400.0, 600.0), (0.5, 0.8), dark, normal),
    smoke(&cloud, (width * 0.7, 0.0, width * 0.3, height), 46, 9, (400.0, 600.0), (0.5, 0.8), dark, normal),
    smoke(&cloud, (width * 0.25, height * 0.02, width * 0.5, height * 0.4), 41, 5, (420.0, 620.0), (0.22, 0.4), (0x7fc4e0, 0xdff4fb), add),
    smoke(&curl, (width * 0.2, 0.0, width * 0.6, height * 0.5), 42, 3, (360.0, 520.0), (0.3, 0.5), (0x7fcbe8, 0xd8f4ff), add),
  ] {
    system.apply(&mut image);
  }
  // The wings: long primaries behind, then shorter rows in front of them.
  for ((from, to), side, seed) in [(left_wing, -1.0, 10), (right_wing, 1.0, 20)] {
    for system in [
      feathers(from, to, side, seed, 20, ((150.0, 330.0), (-340.0, -150.0)), (13.0, 17.0)),
      feathers(from, to, side, seed + 1, 26, ((90.0, 240.0), (-250.0, -90.0)), (14.0, 18.0)),
      feathers(from, to, side, seed + 2, 34, ((40.0, 150.0), (-170.0, -40.0)), (14.0, 19.0)),
    ] {
      system.apply(&mut image);
    }
  }
  // The floor: the cloud bank piled along the bottom, dark where it covers and lit where it is thickest, a pale lip of it on top.
  for system in [
    smoke(&cloud, (0.0, height * 0.66, width, height * 0.45), 51, 16, (420.0, 620.0), (0.45, 0.7), deep, normal),
    smoke(&cloud, (0.0, height * 0.82, width, height * 0.35), 52, 10, (440.0, 640.0), (0.4, 0.65), deep, normal),
    // Lit billows over the dark: the same clouds again, added, so that their bright middles stand out of the dark.
    smoke(&cloud, (0.0, height * 0.6, width, height * 0.25), 53, 8, (360.0, 540.0), (0.1, 0.2), (0x2c7fa6, 0x9fd8ee), add),
  ] {
    system.apply(&mut image);
  }
  // In front: the wispy smoke photo, faint along the floor so that it stays the cloud bank, and up by the wings.
  for system in [
    smoke(&wisp, (0.0, height * 0.1, width * 0.4, height * 0.3), 43, 2, (300.0, 420.0), (0.1, 0.2), (0x8fd4ee, 0xeafaff), add),
    smoke(&wisp, (0.0, height * 0.5, width * 0.5, height * 0.5), 54, 3, (380.0, 520.0), (0.06, 0.12), (0x8fd4ee, 0xeafaff), add),
    smoke(&wisp, (width * 0.6, height * 0.1, width * 0.4, height * 0.3), 50, 2, (300.0, 420.0), (0.1, 0.2), (0x8fd4ee, 0xeafaff), add),
    smoke(&wisp, (width * 0.5, height * 0.5, width * 0.5, height * 0.5), 55, 3, (380.0, 520.0), (0.06, 0.12), (0x8fd4ee, 0xeafaff), add),
  ] {
    system.apply(&mut image);
  }
  image
}

/// A whale landing in the water, as in a photo of one: a billowing white mass of spray, wider than it is tall and in two or three
/// lobes, its edges breaking up into fine droplets, with a bluish shaded underside and a sheet of foam streaming out along the
/// water. It is made in the ocean at the upper left of the picture.
fn splash(p_image: &Image) -> Image {
  let (width, height) = p_image.dimensions::<u32>();
  let (width, height) = (width as f32, height as f32);
  let hit = (width * 0.2, height * 0.265 - 50.0);
  let mist = smoke_texture("assets/smoke/smoke-cloud-bank.png", 25.0, 200.0, 0.8, 0.7);
  let wisp = smoke_texture("assets/smoke/smoke-wisps.png", 35.0, 220.0, 0.9, 0.45);

  // The body of the splash: cloud-bank stamps, each tinted from `tint.0` to `tint.1` and painted over the picture.
  let body = |texture: &Image, seed: u64, rect: (f32, f32, f32, f32), count: usize, size: (f32, f32), opacity: (f32, f32), tint: (u32, u32)| {
    particle_system()
      .with_seed(seed)
      .with_emitter(Emitter::Rect(rect.0, rect.1, rect.2, rect.3))
      .with_count(count)
      .with_size(size)
      .with_opacity(opacity)
      .with_rotation((-30.0, 30.0))
      .with_start_color(Gradient::from_to(Color::from_hex(tint.0), Color::from_hex(tint.1)))
      .with_texture(texture.clone())
      .with_progress(0.5)
  };
  let (grey, white) = ((0x7f9bb4, 0xb9cddd), (0xe8f2f8, 0xffffff));
  // Fine spray: a fog of tiny droplets thrown out and up from the mass, with no two alike.
  let spray = |seed: u64, count: usize, spread: f32, up: (f32, f32), size: (f32, f32), opacity: (f32, f32)| {
    particle_system()
      .with_seed(seed)
      .with_emitter(Emitter::Line((hit.0 - 200.0, hit.1 - 20.0), (hit.0 + 200.0, hit.1 - 20.0)))
      .with_count(count)
      .with_size(size)
      .with_opacity(opacity)
      .with_start_color(Gradient::from_to(Color::from_hex(0xf2f8fc), Color::from_hex(0xffffff)))
      .with_start_speed((-spread, spread), up)
      .with_gravity(900.0)
      .with_noise(30.0, 50.0)
      .with_progress(0.55)
      .with_age_spread(0.45)
  };
  // Foam streaming out along the water: low, wide streaks.
  let foam = |seed: u64, count: usize, reach: f32| {
    particle_system()
      .with_seed(seed)
      .with_emitter(Emitter::Line((hit.0 - 80.0, hit.1 + 10.0), (hit.0 + 80.0, hit.1 + 10.0)))
      .with_count(count)
      .with_size((Curve::from_to(4.0, 1.0), Curve::from_to(7.0, 2.0)))
      .with_opacity((0.35, 0.8))
      .with_start_color(Gradient::from_to(Color::from_hex(0xdff1f8), Color::from_hex(0xffffff)))
      .with_start_speed((-reach, reach), (-30.0, 20.0))
      .with_noise(26.0, 40.0)
      .with_progress(1.0)
      .with_trail(1.0)
      .with_trail_fade(0.3)
      .with_trail_hardness(0.2)
  };

  let mut image = p_image.clone();
  for system in [
    // The shaded underside first, bluish grey, low and wide.
    body(&mist, 70, (hit.0 - 190.0, hit.1 - 70.0, 380.0, 80.0), 8, (80.0, 120.0), (0.5, 0.75), grey),
    // Then the lobes: a big one in the middle and one either side, each a pile of white stamps.
    body(&mist, 71, (hit.0 - 70.0, hit.1 - 190.0, 140.0, 170.0), 9, (95.0, 145.0), (0.55, 0.85), white),
    body(&mist, 72, (hit.0 - 200.0, hit.1 - 120.0, 110.0, 110.0), 6, (75.0, 110.0), (0.5, 0.8), white),
    body(&mist, 73, (hit.0 + 90.0, hit.1 - 130.0, 110.0, 120.0), 6, (75.0, 110.0), (0.5, 0.8), white),
    // A thin pale plume of mist rising beside it, as in the photo of a whale surfacing.
    body(&wisp, 74, (hit.0 - 30.0, hit.1 - 230.0, 140.0, 140.0), 2, (120.0, 180.0), (0.1, 0.2), white),
    // The fringe: the fog of fine spray, and a few bigger drops.
    spray(75, 1800, 300.0, (-640.0, -80.0), (0.8, 1.8), (0.45, 0.9)),
    spray(76, 160, 340.0, (-760.0, -200.0), (2.0, 3.6), (0.7, 1.0)),
    // The foam sheet along the water, over the base of the mass.
    foam(77, 160, 260.0),
    body(&mist, 78, (hit.0 - 200.0, hit.1 - 12.0, 400.0, 30.0), 6, (60.0, 95.0), (0.3, 0.55), white),
  ] {
    system.apply(&mut image);
  }
  image
}

/// What makes one launch trail different from another.
struct TrailLook {
  /// How far the wobble swings, for a trail about 750 px long (it scales with the length). Negative bends the other way first.
  wobble: f32,
  /// Whole swings the wobble makes between the foot and the burst. It has to be a whole number for the trail to end at the burst.
  cycles: f32,
  /// Multiplies the width and the brightness of the line.
  thickness: f32,
  brightness: f32,
  /// How many fine sparks and large beads it sheds, and how big the beads are.
  specks: usize,
  beads: usize,
  bead_size: f32,
  /// Where it leaves the ground, as a step from the foot.
  launch: (f32, f32),
}

/// Fireworks over a darkened copy of the picture: each burst is a ring of glowing streaks thrown out in every direction, drooping under
/// gravity, and each streak crackles into tiny sparkles near the end of its life.
fn fireworks(p_image: &Image) -> Image {
  let (width, height) = p_image.dimensions::<u32>();
  let (width, height) = (width as f32, height as f32);
  let add = Blend::Additive;

  // Night: the day is drained of color and exposed down a few stops, tinted a deep blue by multiplying, and the edges go nearly black.
  let mut image = p_image.clone();
  levels::saturation(-45).apply(&mut image);
  levels::exposure(-2.2).with_gamma(1.15).apply(&mut image);
  let moonlight = Image::new_from_color(width as u32, height as u32, Color::from_hex(0x4a66c8));
  blend(&moonlight).with_mode(BlendMode::Multiply).apply(&mut image);
  let (w, h) = (width as u32, height as u32);
  blend(&color::vignette(w, h, Color::from_rgba(1, 3, 14, 200), 0.3, 1.5)).apply(&mut image);

  // One shell. `p_reach` is how far the longest streaks go; they range from about half of that to all of it, so the shell has a
  // dense ring with a hollow middle. `p_colors` runs along each streak from its start to its end.
  let burst = |center: (f32, f32), seed: u64, count: usize, reach: f32, colors: [u32; 3]| {
    // Sparkles that scatter off a streak when it is nearly spent.
    let crackle = particle_system()
      .with_seed(seed + 1)
      .with_count(2)
      .with_size((0.7, 1.6))
      .with_opacity(Curve::from_to(0.95, 0.0))
      .with_start_color(Gradient::from_to(Color::from_hex(colors[1]), Color::from_hex(0xffffff)))
      .with_start_speed((-60.0, 60.0), (-60.0, 60.0))
      .with_gravity(180.0)
      .with_progress(0.5)
      .with_age_spread(0.5)
      .with_blend(add);
    particle_system()
      .with_seed(seed)
      .with_emitter(Emitter::Circle { center, radius: 4.0 })
      .with_count(count)
      .with_size((Curve::from_to(1.5, 0.5), Curve::from_to(2.3, 0.9)))
      .with_opacity((Curve::from_to(0.55, 0.9), Curve::from_to(0.8, 1.0)))
      .with_color_over_lifetime(Gradient::evenly(colors.iter().map(|hex| Color::from_hex(*hex)).collect()))
      .with_radial_velocity((reach * 0.45, reach))
      .with_gravity(reach * 0.9)
      .with_progress(0.7)
      .with_age_spread(0.05)
      .with_trail(0.45)
      .with_trail_fade(1.0)
      .with_trail_hardness(0.75)
      .with_blend(add)
      .with_sub_emitter(0.8, crackle)
  };
  // The glow a burst throws on the sky, and the bright spot at its middle.
  let glow = |center: (f32, f32), seed: u64, size: f32, color: u32, opacity: f32| {
    particle_system()
      .with_seed(seed)
      .with_emitter(Emitter::Circle { center, radius: 1.0 })
      .with_count(1)
      .with_size(size)
      .with_opacity(opacity)
      .with_start_color(Gradient::from_to(Color::from_hex(color), Color::from_hex(color)))
      .with_blend(add)
  };
  // A shell going up: a wobbling line from the ground to where it bursts. The steady climb is the start speed and the wobble is a cosine
  // velocity of whole cycles, which adds up to a sine that is zero at both ends, so it still starts at the foot and ends at the burst.
  // `p_width` and `p_opacity` make it a soft wide glow or a thin bright core; it is faint at the bottom and brightest at the head.
  let launch = |from: (f32, f32), to: (f32, f32), wobble: &Curve, seed: u64, color: u32, p_width: f32, p_opacity: f32| {
    particle_system()
      .with_seed(seed)
      .with_emitter(Emitter::Circle { center: from, radius: 0.5 })
      .with_count(1)
      .with_size(Curve::from_to(p_width * 0.5, p_width))
      .with_opacity(Curve::from_to(p_opacity * 0.4, p_opacity))
      .with_start_color(Gradient::from_to(Color::from_hex(color), Color::from_hex(0xffffff)))
      .with_start_speed(to.0 - from.0, to.1 - from.1)
      .with_velocity_over_lifetime(wobble.clone(), 0.0)
      .with_progress(1.0)
      .with_trail(1.0)
      .with_trail_fade(0.85)
      .with_trail_hardness(0.6)
      .with_blend(add)
  };
  // Sparks left along the way up: the same climb and wobble as the line, each at its own age so they are spread along it, and each born up to 16 px to one side so they spread out around it. `p_reach` is how much of the climb they cover, counted back from the head.
  let shed = |from: (f32, f32), to: (f32, f32), wobble: &Curve, seed: u64, color: u32, count: usize, size: (f32, f32), reach: f32| {
    particle_system()
      .with_seed(seed)
      .with_emitter(Emitter::Circle { center: from, radius: 16.0 })
      .with_count(count)
      .with_size(size)
      .with_opacity((0.4, 1.0))
      .with_start_color(Gradient::from_to(Color::from_hex(color), Color::from_hex(0xffffff)))
      .with_start_speed(to.0 - from.0, to.1 - from.1)
      .with_velocity_over_lifetime(wobble.clone(), 0.0)
      .with_progress(1.0 - reach / 2.0)
      .with_age_spread(reach / 2.0)
      .with_blend(add)
  };
  // Smoke where the rockets leave the ground, as with the flame: dark puffs that grow and fade as they rise. Each trail has its own, which
  // climbs along that trail (a share of its line, with the same bend), so the smoke hangs on the path the rocket made.
  let puff = smoke_texture("assets/smoke/smoke-cloud-bank.png", 25.0, 200.0, 0.8, 0.7);
  let base_smoke = |from: (f32, f32), to: (f32, f32), wobble: (f32, f32), seed: u64| {
    let climb = 0.42;
    particle_system()
      .with_seed(seed)
      .with_emitter(Emitter::Circle { center: from, radius: 5.0 })
      .with_count(80)
      .with_size((Curve::from_to(18.0, 42.0), Curve::from_to(28.0, 62.0)))
      .with_opacity((Curve::from_to(0.55, 0.0), Curve::from_to(0.9, 0.06)))
      .with_rotation((-40.0, 40.0))
      .with_color_over_lifetime(Gradient::evenly(vec![Color::from_hex(0x121212), Color::from_hex(0x1b1c22), Color::from_hex(0x2c3040)]))
      .with_texture(puff.clone())
      .with_start_speed((to.0 - from.0) * climb, (to.1 - from.1) * climb)
      .with_velocity_over_lifetime(Curve::cosine(wobble.0 * climb, wobble.1), 0.0)
      .with_progress(0.5)
      .with_age_spread(0.5)
  };
  // Drifting smoke from earlier shells, low in the sky behind the bursts.
  let mist = smoke_texture("assets/smoke/smoke-wisps.png", 35.0, 220.0, 0.9, 0.45);
  let haze = |seed: u64, rect: (f32, f32, f32, f32), color: u32| {
    particle_system()
      .with_seed(seed)
      .with_emitter(Emitter::Rect(rect.0, rect.1, rect.2, rect.3))
      .with_count(3)
      .with_size((300.0, 460.0))
      .with_opacity((0.05, 0.1))
      .with_rotation((-20.0, 20.0))
      .with_start_color(Gradient::from_to(Color::from_hex(color), Color::from_hex(color)))
      .with_texture(mist.clone())
      .with_blend(add)
      .with_progress(0.5)
  };
  // Stray sparks: single bright points high in the sky, as in the last of the references.
  let sparks = particle_system()
    .with_seed(400)
    .with_emitter(Emitter::Rect(0.0, 0.0, width, height * 0.45))
    .with_count(26)
    .with_size((1.2, 2.4))
    .with_opacity((0.5, 1.0))
    .with_start_color(Gradient::evenly(vec![Color::from_hex(0xff5a8a), Color::from_hex(0xfff0b0), Color::from_hex(0x9fdcff)]))
    .with_blend(add)
    .with_progress(0.5);

  // Her feet in the picture (left, right), where every shell is launched from, and the center of each of the ten bursts.
  let feet = ((width * 0.485, height * 0.93), (width * 0.6, height * 0.935));
  // A look is: wobble, cycles, thickness, brightness, specks, beads, bead size, and where it leaves the ground.
  let look = |wobble, cycles, thickness, brightness, specks, beads, bead_size, launch| TrailLook { wobble, cycles, thickness, brightness, specks, beads, bead_size, launch };
  let shells = [
    ((width * 0.22, height * 0.2), 100, 520, 290.0, [0xfff3c0, 0xffb830, 0xb8400c], feet.0, look(110.0, 3.0, 1.0, 1.0, 520, 40, 3.5, (-6.0, 4.0))),
    ((width * 0.84, height * 0.23), 200, 460, 230.0, [0xffe6ff, 0xff4fa8, 0x5a1bd6], feet.1, look(-170.0, 2.0, 1.5, 1.0, 380, 70, 4.4, (30.0, 8.0))),
    ((width * 0.4, height * 0.08), 300, 360, 170.0, [0xe8fdff, 0x35b6ff, 0x1e3ed6], feet.0, look(60.0, 4.0, 0.7, 0.85, 640, 26, 3.0, (-44.0, 14.0))),
    ((width * 0.09, height * 0.43), 1000, 170, 120.0, [0xf0ffe0, 0x6dff5a, 0x0f8a2a], feet.0, look(-230.0, 3.0, 0.9, 0.8, 260, 18, 3.8, (-80.0, 22.0))),
    ((width * 0.64, height * 0.07), 1100, 220, 150.0, [0xfff0e0, 0xff4a3a, 0x9a0f1a], feet.1, look(150.0, 5.0, 1.2, 1.0, 480, 54, 3.2, (8.0, -4.0))),
    ((width * 0.96, height * 0.07), 1200, 190, 125.0, [0xffffff, 0xc8d8ff, 0x6a78b0], feet.1, look(-90.0, 4.0, 0.6, 0.75, 700, 14, 2.8, (60.0, 20.0))),
    ((width * 0.72, height * 0.44), 1300, 200, 140.0, [0xe0fff8, 0x35e6c8, 0x0b6a8a], feet.1, look(210.0, 2.0, 1.3, 0.9, 300, 60, 4.2, (-16.0, 16.0))),
    ((width * 0.35, height * 0.42), 1400, 180, 130.0, [0xf0e0ff, 0xa060ff, 0x4a1aa0], feet.0, look(-70.0, 3.0, 0.8, 0.9, 560, 22, 3.4, (24.0, 10.0))),
    ((width * 0.12, height * 0.05), 1500, 190, 125.0, [0xfffbe0, 0xffe04a, 0xb87a0a], feet.0, look(130.0, 5.0, 1.0, 0.7, 420, 36, 3.6, (-60.0, 6.0))),
    ((width * 0.92, height * 0.45), 1600, 150, 110.0, [0xffe8f0, 0xff7ab0, 0x9a1a5a], feet.1, look(-190.0, 3.0, 1.1, 1.0, 340, 48, 4.0, (44.0, 2.0))),
  ];
  haze(500, (0.0, height * 0.18, width * 0.45, height * 0.25), 0xa08060).apply(&mut image);
  haze(501, (width * 0.55, height * 0.2, width * 0.45, height * 0.25), 0x9060a0).apply(&mut image);
  for (center, seed, count, reach, colors, foot, look) in &shells {
    let (center, seed, count, reach, colors) = (*center, *seed, *count, *reach, *colors);
    let from = (foot.0 + look.launch.0, foot.1 + look.launch.1);
    // The wobble grows with the length of the trail, so a short trail does not swing as hard as a long one.
    let length = ((center.0 - from.0).powi(2) + (center.1 - from.1).powi(2)).sqrt();
    let swing = look.wobble * length / 750.0;
    let wobble = Curve::cosine(swing, look.cycles);
    launch(from, center, &wobble, seed + 20, colors[2], 11.0 * look.thickness, 0.4 * look.brightness).apply(&mut image);
    // Mostly fine specks along most of the climb, and a few large beads kept toward the head.
    shed(from, center, &wobble, seed + 22, colors[1], look.specks, (0.5, 1.6), 0.8).apply(&mut image);
    shed(from, center, &wobble, seed + 24, colors[0], look.beads, (look.bead_size * 0.7, look.bead_size * 1.25), 0.4).apply(&mut image);
    launch(from, center, &wobble, seed + 21, colors[0], 3.6 * look.thickness, (0.95 * look.brightness).min(1.0)).apply(&mut image);
    glow(center, seed + 23, 7.0, 0xffffff, 0.9).apply(&mut image);
    glow(center, seed + 9, reach * 1.5, colors[2], 0.2).apply(&mut image);
    glow(center, seed + 10, reach * 0.18, colors[0], 0.7).apply(&mut image);
    burst(center, seed, count, reach, colors).apply(&mut image);
  }
  sparks.apply(&mut image);
  // The smoke goes on last, in front of the trails and their sparks, so the lines come up out of it and are partly hidden by it.
  // One smoke in two colors: dark charcoal at the base, turning a cooler blue-grey as it rises and thins.
  for (center, seed, _, _, _, foot, look) in &shells {
    let from = (foot.0 + look.launch.0, foot.1 + look.launch.1);
    let length = ((center.0 - from.0).powi(2) + (center.1 - from.1).powi(2)).sqrt();
    base_smoke(from, *center, (look.wobble * length / 750.0, look.cycles), seed + 30).apply(&mut image);
  }
  image
}
