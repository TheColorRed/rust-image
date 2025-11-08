use abra::{
  color::Color,
  draw::line,
  draw::shapes,
  geometry::{
    path::{BezierCurve, Path, Rect},
    point::Point,
  },
  image::Image,
};

pub fn main() {
  let mut image = Image::new_from_path("assets/bikini.jpg");
  let (width, height) = image.dimensions::<u32>();
  let start_time = std::time::Instant::now();

  let mut curve = <Path as BezierCurve>::new_curve();
  curve.add_three_point(Point::new(0, 0), Point::new(100, 100), Point::new(200, 0));

  line::curve_from(&mut image, Point::new(100, 200), curve, Color::from_rgba(255, 0, 0, 128));

  // let r = Rect::new_rect(100, 100);

  // shapes::circle_stroke(
  //   &mut image,
  //   Point::new((width / 2) as i32, (height / 2) as i32),
  //   15,
  //   Color::red(),
  //   2,
  // );
  // shapes::rect(
  //   &mut image,
  //   Point::new(100, 100),
  //   Rect::new_rect(20, 200),
  //   Color::from_rgba(0, 0, 255, 128),
  // );

  // shapes::circle(&mut image, Point::new(400, 200), 50, Color::from_rgba(0, 255, 255, 128));

  // shapes::ellipse_filled(
  //   &mut image,
  //   Point::new((width / 2) as i32, (height / 2) as i32),
  //   Rect::new_rect(100, 200),
  //   Color::from_rgba(255, 0, 255, 128),
  // );

  // shapes::polygon(
  //   &mut image,
  //   Point::new(200, 100),
  //   // make a 5 pointed star with 10 points
  //   Path::new(vec![
  //     (0, 0),
  //     (10, 30),
  //     (40, 30),
  //     (20, 50),
  //     (30, 80),
  //     (0, 60),
  //     (-30, 80),
  //     (-20, 50),
  //     (-40, 30),
  //     (-10, 30),
  //   ]),
  //   Color::from_rgba(255, 0, 0, 128),
  // );

  println!("Time: {:?}", start_time.elapsed());

  image.save("out/rect.png", None);
}
