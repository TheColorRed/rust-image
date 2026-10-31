use abra::prelude::*;
use abra_particle_system::prelude::*;

const SIZE: u32 = 100;

fn black() -> Image {
  Image::new_from_color(SIZE, SIZE, Color::black())
}

/// A system of single-pixel white dots that stay where they are born, to be changed by the test.
fn dots() -> ParticleSystem {
  particle_system()
    .with_count(300)
    .with_size(1.0)
    .with_opacity(1.0)
    .with_start_color(Gradient::from_to(Color::white(), Color::white()))
    .with_progress(1.0)
}

/// The pixels the particles lit, as `(x, y)`.
fn lit(p_image: &Image) -> Vec<(f32, f32)> {
  let mut points = Vec::new();
  for y in 0..SIZE {
    for x in 0..SIZE {
      let (r, g, b, _) = p_image.get_pixel(x, y).unwrap();
      if r.max(g).max(b) > 100 {
        points.push((x as f32, y as f32));
      }
    }
  }
  points
}

fn distance(p_a: (f32, f32), p_b: (f32, f32)) -> f32 {
  ((p_a.0 - p_b.0).powi(2) + (p_a.1 - p_b.1).powi(2)).sqrt()
}

#[test]
fn the_same_seed_gives_the_same_picture() {
  let system = dots().with_seed(5).with_noise(10.0, 20.0);
  assert_eq!(system.applied(&black()).rgba(), system.applied(&black()).rgba());
  assert_ne!(system.applied(&black()).rgba(), system.clone().with_seed(6).applied(&black()).rgba());
}

#[test]
fn a_ring_emits_between_its_radii() {
  let image = dots().with_emitter(Emitter::Ring { center: (50.0, 50.0), radius: 30.0, inner_radius: 20.0 }).applied(&black());
  let points = lit(&image);
  assert!(!points.is_empty());
  assert!(points.iter().all(|p| (17.0..=33.0).contains(&distance(*p, (50.0, 50.0)))));
}

#[test]
fn a_line_emits_along_the_line() {
  let image = dots().with_emitter(Emitter::Line((10.0, 20.0), (90.0, 20.0))).applied(&black());
  let points = lit(&image);
  assert!(!points.is_empty());
  assert!(points.iter().all(|p| (p.1 - 20.0).abs() <= 2.0 && (8.0..=92.0).contains(&p.0)));
}

#[test]
fn a_luminance_emitter_emits_only_from_the_light_parts() {
  let mut base = black();
  for y in 70..90 {
    for x in 70..90 {
      base.set_pixel(x, y, (255, 255, 255, 255));
    }
  }
  // Red on the white square shows where particles are born.
  let image = dots()
    .with_emitter(Emitter::Luminance(0.5))
    .with_start_color(Gradient::from_to(Color::from_rgb(255, 0, 0), Color::from_rgb(255, 0, 0)))
    .applied(&base);
  let mut red = 0;
  for y in 0..SIZE {
    for x in 0..SIZE {
      let (r, g, _, _) = image.get_pixel(x, y).unwrap();
      if r > 100 && g < 50 {
        red += 1;
        assert!((68..92).contains(&x) && (68..92).contains(&y), "a particle at ({x}, {y}) is outside the light square");
      }
    }
  }
  assert!(red > 0, "no particles were drawn");
}

#[test]
fn radial_velocity_pushes_particles_away_from_the_center() {
  let image = dots()
    .with_emitter(Emitter::Circle { center: (50.0, 50.0), radius: 5.0 })
    .with_radial_velocity(20.0)
    .applied(&black());
  let points = lit(&image);
  assert!(!points.is_empty());
  assert!(points.iter().all(|p| distance(*p, (50.0, 50.0)) > 15.0), "particles did not move out");
}

#[test]
fn orbital_velocity_turns_particles_around_the_center() {
  let image = dots()
    .with_emitter(Emitter::Line((70.0, 50.0), (70.0, 50.0)))
    .with_orbital_velocity(90.0)
    .with_velocity_center(50.0, 50.0)
    .applied(&black());
  let points = lit(&image);
  assert!(!points.is_empty());
  // A quarter turn takes (70, 50), 20 to the right of the center, to 20 below it.
  assert!(points.iter().all(|p| distance(*p, (50.0, 70.0)) <= 3.0), "particles did not quarter turn");
}

#[test]
fn gravity_pulls_particles_down() {
  let image = dots().with_emitter(Emitter::Line((20.0, 10.0), (80.0, 10.0))).with_gravity(40.0).applied(&black());
  let points = lit(&image);
  assert!(!points.is_empty());
  // Half of 40 over a whole life squared is 20 pixels.
  assert!(points.iter().all(|p| (p.1 - 30.0).abs() <= 3.0));
}

#[test]
fn hue_variation_turns_the_color() {
  let image = dots()
    .with_emitter(Emitter::Line((20.0, 50.0), (80.0, 50.0)))
    .with_start_color(Gradient::from_to(Color::from_rgb(255, 0, 0), Color::from_rgb(255, 0, 0)))
    .with_hue_variation(120.0)
    .applied(&black());
  let points = lit(&image);
  assert!(!points.is_empty());
  // Red turned a third of the way round the wheel is green.
  assert!(points.iter().all(|(x, y)| {
    let (r, g, _, _) = image.get_pixel(*x as u32, *y as u32).unwrap();
    g > r
  }));
}

#[test]
fn noise_moves_particles_and_leaves_newborns_alone() {
  let still = dots().with_emitter(Emitter::Line((20.0, 50.0), (80.0, 50.0)));
  let blown = still.clone().with_noise(30.0, 15.0);
  assert_ne!(still.applied(&black()).rgba(), blown.applied(&black()).rgba());
  let born = blown.with_progress(0.0);
  assert!(lit(&born.applied(&black())).iter().all(|p| (p.1 - 50.0).abs() <= 2.0), "noise moved particles at age 0");
}

#[test]
fn a_blend_mode_is_used() {
  let mut base = black();
  base.clear_color(Color::from_rgb(200, 200, 200));
  // White painted on gray covers it with Normal and leaves it with Multiply.
  let normal = dots().with_blend(BlendMode::Normal).applied(&base);
  let multiplied = dots().with_blend(BlendMode::Multiply).applied(&base);
  assert!(normal.rgba().chunks(4).any(|pixel| pixel[0] > 240));
  assert!(multiplied.rgba().chunks(4).all(|pixel| pixel[0] == 200));
}

#[test]
fn additive_adds_light_scaled_by_opacity() {
  let mut base = black();
  base.clear_color(Color::from_rgb(100, 100, 100));
  let image = dots().with_opacity(0.5).with_blend(Blend::Additive).applied(&base);
  // White at half opacity adds about 128 to 100, and no pixel passes 255 or goes below what was there.
  assert!(image.rgba().chunks(4).any(|pixel| (220..=235).contains(&pixel[0])));
  assert!(image.rgba().chunks(4).all(|pixel| pixel[0] >= 100));
}

/// One particle at the middle of the image, size 10, with the given texture and rotation.
fn stamp(p_texture: Image, p_rotation: f32) -> Image {
  particle_system()
    .with_count(1)
    .with_emitter(Emitter::Line((50.0, 50.0), (50.0, 50.0)))
    .with_size(10.0)
    .with_opacity(1.0)
    .with_progress(1.0)
    .with_rotation(p_rotation)
    .with_texture(p_texture)
    .applied(&black())
}

fn red_square() -> Image {
  Image::new_from_color(8, 8, Color::from_rgb(255, 0, 0))
}

#[test]
fn a_texture_is_drawn_instead_of_the_dot() {
  let image = stamp(red_square(), 0.0);
  // A square twice the size across: red inside it, and black outside, where a dot would have faded out.
  assert_eq!(image.get_pixel(50, 50).map(|p| p.0), Some(255));
  assert_eq!(image.get_pixel(58, 58).map(|p| (p.0, p.1)), Some((255, 0)));
  assert_eq!(image.get_pixel(62, 50).map(|p| p.0), Some(0));
  assert_eq!(image.get_pixel(50, 62).map(|p| p.0), Some(0));
}

#[test]
fn rotating_a_texture_turns_it_around_its_middle() {
  let (flat, turned) = (stamp(red_square(), 0.0), stamp(red_square(), 45.0));
  // The corner of the flat square is out of the diamond a quarter turn makes, and the diamond's point is out past the flat edge.
  assert_eq!(flat.get_pixel(58, 58).map(|p| p.0), Some(255));
  assert_eq!(turned.get_pixel(58, 58).map(|p| p.0), Some(0));
  assert_eq!(flat.get_pixel(62, 50).map(|p| p.0), Some(0));
  assert_eq!(turned.get_pixel(62, 50).map(|p| p.0), Some(255));
}

#[test]
fn a_texture_keeps_its_shape_and_its_see_through_parts() {
  // Twice as wide as it is tall: the long side is twice the size, and the short side half of that.
  let wide = Image::new_from_color(8, 4, Color::from_rgb(255, 0, 0));
  let image = stamp(wide, 0.0);
  assert_eq!(image.get_pixel(58, 50).map(|p| p.0), Some(255));
  assert_eq!(image.get_pixel(50, 58).map(|p| p.0), Some(0));
  let clear = stamp(Image::new_from_color(8, 8, Color::transparent()), 0.0);
  assert!(clear.rgba().chunks(4).all(|pixel| pixel[0] == 0));
}

#[test]
fn a_particles_color_tints_the_texture() {
  let white = Image::new_from_color(8, 8, Color::white());
  let tinted = particle_system()
    .with_count(1)
    .with_emitter(Emitter::Line((50.0, 50.0), (50.0, 50.0)))
    .with_size(10.0)
    .with_opacity(1.0)
    .with_start_color(Gradient::from_to(Color::from_rgb(255, 0, 0), Color::from_rgb(255, 0, 0)))
    .with_texture(white.clone())
    .applied(&black());
  assert_eq!(tinted.get_pixel(50, 50).map(|p| (p.0, p.1, p.2)), Some((255, 0, 0)));
  // With no color the texture is drawn as it is, not as the pixel it was born on.
  let plain = stamp(white, 0.0);
  assert_eq!(plain.get_pixel(50, 50).map(|p| (p.0, p.1, p.2)), Some((255, 255, 255)));
}

/// One particle at (50, 50) that is half way through a life of 60 pixels straight down, so its head is at y 80.
fn falling() -> ParticleSystem {
  particle_system()
    .with_count(1)
    .with_emitter(Emitter::Line((50.0, 50.0), (50.0, 50.0)))
    .with_size(1.0)
    .with_opacity(1.0)
    .with_start_color(Gradient::from_to(Color::white(), Color::white()))
    .with_start_speed(0.0, 60.0)
    .with_progress(0.5)
}

#[test]
fn a_trail_is_left_along_the_path_and_fades_toward_the_tail() {
  let brightness = |image: &Image, y: u32| image.get_pixel(50, y).unwrap().0;
  let bare = falling().applied(&black());
  assert_eq!(brightness(&bare, 65), 0, "without a trail nothing is left behind");
  let trailed = falling().with_trail(0.5).applied(&black());
  assert!(brightness(&trailed, 80) > 200, "the head is bright");
  assert!(brightness(&trailed, 65) > 20, "the streak reaches back along the path");
  assert!(brightness(&trailed, 55) < brightness(&trailed, 75), "the far end is fainter than the near end");
  assert!(brightness(&trailed, 52) < 25, "the tail fades out to nothing");
  assert_eq!(brightness(&trailed, 40), 0, "nothing is drawn before the particle was born");
  assert_eq!(brightness(&trailed, 92), 0, "nothing is drawn ahead of the head");
}

#[test]
fn a_trail_takes_the_colors_the_particle_had_earlier() {
  let image = falling()
    .with_color_over_lifetime(Gradient::from_to(Color::from_rgb(255, 0, 0), Color::from_rgb(0, 0, 255)))
    .with_trail(0.5)
    .applied(&black());
  // The tail is faint, so what is compared is the mix of colors and not how bright it is.
  let blue_share = |y: u32| {
    let (r, _, b, _) = image.get_pixel(50, y).unwrap();
    b as f32 / (r as f32 + b as f32).max(1.0)
  };
  assert!(blue_share(56) < 0.2, "the tail has the earlier, red color: {}", blue_share(56));
  assert!(blue_share(80) > 0.35, "the head has the later, bluer color: {}", blue_share(80));
  assert!(blue_share(56) < blue_share(68) && blue_share(68) < blue_share(80), "the color changes along the trail");
}

#[test]
fn a_trail_that_does_not_fade_stays_as_strong_at_its_tail() {
  let brightness = |image: &Image, y: u32| image.get_pixel(50, y).unwrap().0;
  let faded = falling().with_trail(0.5).applied(&black());
  let solid = falling().with_trail(0.5).with_trail_fade(0.0).applied(&black());
  assert!(brightness(&faded, 53) < 20, "a fading trail is nearly gone at its tail");
  assert!(brightness(&solid, 53) > 40, "a trail that does not fade is still there at its tail");
  assert!(brightness(&solid, 53) < brightness(&solid, 78) + 40);
}

#[test]
fn a_soft_dot_is_not_cut_off_before_it_fades_out() {
  // Size 20 fades out at 30 pixels from the middle; the pixels between 20 and 30 are still painted.
  let image = particle_system()
    .with_count(1)
    .with_emitter(Emitter::Line((50.0, 50.0), (50.0, 50.0)))
    .with_size(20.0)
    .with_opacity(1.0)
    .with_start_color(Gradient::from_to(Color::white(), Color::white()))
    .applied(&Image::new_from_color(120, 120, Color::black()));
  assert!(image.get_pixel(50 + 24, 50).unwrap().0 > 10, "painted past the size");
  assert_eq!(image.get_pixel(50 + 34, 50).unwrap().0, 0, "nothing past one and a half times the size");
}

#[test]
fn gravity_does_not_change_which_way_radial_velocity_pushes() {
  // A shell thrown out from the middle goes out in every direction, and gravity then drops all of it by the same amount.
  let shell = |gravity: f32| {
    lit(
      &dots()
        .with_emitter(Emitter::Circle { center: (50.0, 40.0), radius: 3.0 })
        .with_count(400)
        .with_radial_velocity(30.0)
        .with_gravity(gravity)
        .applied(&black()),
    )
  };
  let (still, falling) = (shell(0.0), shell(20.0));
  let spread = |points: &[(f32, f32)]| {
    let (up, down) = (points.iter().filter(|p| p.1 < 40.0).count(), points.iter().filter(|p| p.1 > 40.0).count());
    (up, down)
  };
  let (up, down) = spread(&still);
  assert!(up > 100 && down > 100, "radial velocity goes out every way: {up} up, {down} down");
  // Gravity of 20 drops everything 10 pixels, so the shell is the same shape lower down; some of it is still above the middle.
  let (up_falling, down_falling) = spread(&falling);
  assert!(up_falling > 60 && down_falling > up_falling, "{up_falling} up, {down_falling} down");
  let mean = |points: &[(f32, f32)]| points.iter().map(|p| p.1).sum::<f32>() / points.len() as f32;
  assert!((mean(&falling) - mean(&still) - 10.0).abs() < 1.5, "{} vs {}", mean(&falling), mean(&still));
}
