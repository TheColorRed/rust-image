use super::point::Point;

#[derive(Clone)]
/// A path is a list of points that can be used to follow.
pub struct Path {
  /// The points of the path.
  points: Vec<Point>,
}

impl Path {
  /// Creates a new path with no points.
  pub fn default() -> Path {
    Path { points: Vec::new() }
  }

  /// Creates a new path with the given points.
  pub fn new(points: Vec<Point>) -> Path {
    Path { points }
  }

  /// Creates a new path with a line from the given points.
  pub fn line(x1: i32, y1: i32, x2: i32, y2: i32) -> Path {
    let mut path = Path::default();
    path.add_point(x1, y1).add_point(x2, y2);
    path
  }

  /// Creates a new path from the given points.
  pub fn new_from_points(points: Vec<Point>) -> Path {
    let mut path = Path::default();
    for point in points {
      path.add_point(point.x(), point.y());
    }
    path
  }

  /// Adds a point to the path.
  pub fn add_point(&mut self, x: i32, y: i32) -> &mut Self {
    self.points.push(Point::new(x, y));
    self
  }

  /// Adds multiple points to the path.
  pub fn add_points(&mut self, points: Vec<Point>) -> &mut Self {
    self.points.extend(points);
    self
  }

  /// Gets the points of the path.
  pub fn get_points(&self) -> &Vec<Point> {
    &self.points
  }

  /// Gets a point at the given index.
  pub fn get_point_at(&self, index: usize) -> Point {
    self.points[index]
  }

  /// Gets the first point of the path.
  pub fn first(&self) -> Point {
    self.points[0]
  }

  /// Gets the last point of the path.
  pub fn last(&self) -> Point {
    self.points[self.points.len() - 1]
  }

  /// Gets the closest point on the path to the given point.
  pub fn closest_point(&self, x: i32, y: i32) -> Point {
    let mut min_distance = f32::MAX;
    let mut closest_point = Point::default();
    for point in self.points.iter() {
      let distance = ((x - point.x()).pow(2) + (y - point.y()).pow(2)) as f32;
      if distance < min_distance {
        min_distance = distance;
        closest_point = *point;
      }
    }
    closest_point
  }

  /// Gets the closest time on the path to the given point.
  pub fn closest_time(&self, x: f32, y: f32) -> f32 {
    let mut min_distance = f32::MAX;
    let mut closest_t = 0.0;
    let path_points = self.get_points();
    for j in 0..path_points.len() - 1 {
      let point1 = path_points[j];
      let point2 = path_points[j + 1];
      let (x1, y1, x2, y2) = (point1.x() as f32, point1.y() as f32, point2.x() as f32, point2.y() as f32);

      let dx = (x2 - x1) as f32;
      let dy = (y2 - y1) as f32;
      let length_squared = dx * dx + dy * dy;

      let t = (((x - x1) * dx + ((y) - y1) * dy) / length_squared).clamp(0.0, 1.0);
      let px = x1 + t * dx;
      let py = y1 + t * dy;

      let distance = ((x - px).powi(2) + ((y) - py).powi(2)).sqrt();
      if distance < min_distance {
        min_distance = distance;
        closest_t = t;
      }
    }
    closest_t
  }
}

/// A rectangle is a path with four points.
pub trait Rect {
  /// Creates a new rectangle with the given width and height.
  fn new_rect(width: u32, height: u32) -> Self;
  /// Gets the width of the rectangle.
  fn width(&self) -> u32;
  /// Gets the height of the rectangle.
  fn height(&self) -> u32;
  /// Gets the dimensions of the rectangle.
  fn dimensions(&self) -> (u32, u32) {
    (self.width(), self.height())
  }
}

impl Rect for Path {
  fn new_rect(width: u32, height: u32) -> Self {
    let mut path = Path::default();
    path
      .add_point(0, 0)
      .add_point(width as i32, 0)
      .add_point(width as i32, height as i32)
      .add_point(0, height as i32);
    path
  }

  fn width(&self) -> u32 {
    let mut max_x = 0;
    for point in self.points.iter() {
      let x = point.x();
      if x > max_x {
        max_x = x;
      }
    }
    max_x as u32
  }

  fn height(&self) -> u32 {
    let mut max_y = 0;
    for point in self.points.iter() {
      let y = point.y();
      if y > max_y {
        max_y = y;
      }
    }
    max_y as u32
  }
}

#[derive(Clone, PartialEq)]
/// The type of controls for the curve.
/// - If the total number of points is divisible by 3, then the curve has three control points.
/// - If the total number of points is divisible by 4, then the curve has four control points.
pub enum CurveType {
  /// The curve has three control points.
  Three,
  /// The curve has four control points.
  Four,
}

/// A bezier curve is a path with control points.
/// - A three point curve has three control points: start, control, and end.
/// - A four point curve has four control points: start, control1, control2, and end.
/// When a line is drawn, the line will always pass through the start and end points.
pub trait BezierCurve {
  /// Creates a new bezier curve.
  fn new_curve() -> Self;
  /// Creates a new bezier curve from the given points.
  /// - `points` - The points of the curve.
  /// Returns the bezier curve.
  fn from(points: Vec<Point>) -> Self;
  /// Adds a three point curve to the path.
  /// - `start` - The starting point of the curve.
  /// - `control` - The control point between the start and end.
  /// - `end` - The ending point of the curve.
  fn add_three_point(&mut self, start: Point, control: Point, end: Point) -> &mut Self;
  /// Adds a four point curve to the path.
  /// - `start` - The starting point for the curve.
  /// - `control1` - The first control point between the start and end.
  /// - `control2` - The second control point between the start and end.
  /// - `end` - The ending point for the curve.
  fn add_four_point(&mut self, start: Point, control1: Point, control2: Point, end: Point) -> &mut Self;
  /// Gets the point at the given index.
  /// - `index` - The index of the point.
  /// Returns the `point`, `control1`, and `control2` at the given index.
  fn get_three_point_curve_at_index(&self, index: usize) -> (Point, Point, Point);
  /// Gets the point at the given index for a four point curve.
  /// - `index` - The index of the point.
  /// Returns the `point`, `control1`, `control2`, and `end` at the given index.
  fn get_four_point_curve_at_index(&self, index: usize) -> (Point, Point, Point, Point);
  /// Gets the location of where the line is at the given time using the points and control points.
  /// - `time` - The time between 0 and 1 to get the point along the curve.
  /// - `curve_type` - The type of controls for the curve.
  /// Returns the point at the given time.
  fn get_at_time(&self, time: f32, curve_type: CurveType) -> Point;
  /// Gets the type of controls for the curve.
  /// - If the total number of points is divisible by 3, then the curve has three control points.
  /// - If the total number of points is divisible by 4, then the curve has four control points.
  fn get_type(&self) -> CurveType;
  /// Gets the points of the curve.
  fn get_points(&self) -> Vec<Point>;
}

impl BezierCurve for Path {
  fn new_curve() -> Self {
    Path::default()
  }

  fn from(points: Vec<Point>) -> Self {
    let mut path = Path::default();
    path.add_points(points);
    path
  }

  fn add_three_point(&mut self, start: Point, control: Point, end: Point) -> &mut Self {
    self.points.push(start);
    self.points.push(control);
    self.points.push(end);
    self
  }

  fn add_four_point(&mut self, start: Point, control1: Point, control2: Point, end: Point) -> &mut Self {
    self.points.push(start);
    self.points.push(control1);
    self.points.push(control2);
    self.points.push(end);
    self
  }

  fn get_three_point_curve_at_index(&self, index: usize) -> (Point, Point, Point) {
    let point = self.points[index];
    let control1 = self.points[index + 1];
    let control2 = self.points[index + 2];
    (point, control1, control2)
  }

  fn get_four_point_curve_at_index(&self, index: usize) -> (Point, Point, Point, Point) {
    let point = self.points[index];
    let control1 = self.points[index + 1];
    let control2 = self.points[index + 2];
    let end = self.points[index + 3];
    (point, control1, control2, end)
  }

  fn get_at_time(&self, time: f32, curve_type: CurveType) -> Point {
    let mut x = 0;
    let mut y = 0;
    let points = self.get_points();
    // If the curve has three control points per segment
    if curve_type == CurveType::Three {
      // Loop over the points and increment by 3
      for i in 0..points.iter().step_by(3).len() {
        let (point, control1, control2) = self.get_three_point_curve_at_index(i);
        let t = time;
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;
        let p = point * uuu;
        let p1 = control1 * 3.0 * uu * t;
        let p2 = control2 * 3.0 * u * tt;
        let p3 = points[i + 1] * ttt;
        let new_point = p + p1 + p2 + p3;
        x = new_point.x();
        y = new_point.y();
      }
    }
    // If the curve has four control points per segment
    else if curve_type == CurveType::Four {
      for i in 0..points.len() - 1 {
        let (point, control1, control2) = self.get_three_point_curve_at_index(i);
        let t = time;
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;
        let p = point * uuu;
        let p1 = control1 * 3.0 * uu * t;
        let p2 = control2 * 3.0 * u * tt;
        let p3 = points[i + 1] * ttt;
        let new_point = p + p1 + p2 + p3;
        x = new_point.x();
        y = new_point.y();
      }
    }
    Point::new(x, y)
  }

  fn get_type(&self) -> CurveType {
    if self.points.len() % 4 == 0 {
      CurveType::Four
    } else if self.points.len() % 3 == 0 {
      CurveType::Three
    } else {
      panic!("Invalid number of points for bezier curve");
    }
  }

  fn get_points(&self) -> Vec<Point> {
    self.points.clone()
  }
}
