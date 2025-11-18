use crate::utils::as_ui_image;
use crate::{AppWindow, ColorPanel, ColorType};
use abra::draw::line;
use abra::{
  color::{gradient::Gradient, Color},
  draw::gradient::linear_gradient,
  geometry::{Path, Point},
  Image,
};
use slint::{Global, Weak};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, PartialEq)]
pub enum Direction {
  Horizontal,
  Vertical,
}

const HUE_BAR_WIDTH: u32 = 24;
const HUE_BAR_HEIGHT: u32 = 250;
const COLOR_BOX_WIDTH: u32 = 250;
const COLOR_BOX_HEIGHT: u32 = 250;

struct ColorPanelImages {
  pub hue_bar: RefCell<Image>,
  pub box_background: RefCell<Image>,
  pub box_foreground: RefCell<Image>,
  pub black_circle: RefCell<Image>,
  pub white_circle: RefCell<Image>,
}

pub fn entry(app_window: Weak<AppWindow>) {
  let color_panel_images = ColorPanelImages {
    hue_bar: RefCell::new(hue_selector(Point::new(0, 0), Direction::Vertical)),
    box_background: RefCell::new(Image::new(COLOR_BOX_WIDTH, COLOR_BOX_HEIGHT)),
    box_foreground: RefCell::new(trans_to_black_gradient(COLOR_BOX_WIDTH, COLOR_BOX_HEIGHT)),
    black_circle: RefCell::new(circle_image(6, Color::black())),
    white_circle: RefCell::new(circle_image(6, Color::white())),
  };

  let images_ref = Rc::new(RefCell::new(color_panel_images));

  set_ui(app_window.clone(), images_ref.clone());

  let app_weak = app_window.clone();
  let hue_colors = Gradient::hue();
  let images = images_ref.clone();
  ColorPanel::get(&app_window.clone().unwrap()).on_hue_changed(move |hue, x, y| {
    let start_time = std::time::Instant::now();
    let app = app_weak.unwrap();
    let panel = ColorPanel::get(&app);
    let color = hue_colors.get_color_type(hue);
    let (point_x, point_y) = (x * COLOR_BOX_WIDTH as f32, y * COLOR_BOX_HEIGHT as f32);
    let point_color = point_to_color(hue, x, y);
    color_box(images.clone(), color, Point::new(point_x as i32, point_y as i32));

    let images = images.clone();
    let slint_image = as_ui_image(&images.borrow().box_background.borrow());
    panel.set_color_box_background(slint_image);
    panel.set_fg_color(slint::Color::from_rgb_u8(point_color.r, point_color.g, point_color.b));
    println!("Hue changed: {:?}", start_time.elapsed());
  });

  let app_weak = app_window.clone();
  let images = images_ref.clone();
  ColorPanel::get(&app_window.clone().unwrap()).on_color_changed(move |hue, x, y, color_type| {
    let app = app_weak.clone().unwrap();
    let panel = ColorPanel::get(&app);
    let (point_x, point_y) = (x * COLOR_BOX_WIDTH as f32, y * COLOR_BOX_HEIGHT as f32);
    let point_color = point_to_color(hue, x, y);

    let images = images.clone();

    if color_type == ColorType::Foreground {
      panel.set_fg_color(slint::Color::from_rgb_u8(point_color.r, point_color.g, point_color.b));
    } else {
      panel.set_bg_color(slint::Color::from_rgb_u8(point_color.r, point_color.g, point_color.b));
    }

    set_cursor_color(app_weak.clone(), images.clone(), point_color.clone());
    let (cursor_width, cursor_height) = images.borrow().black_circle.borrow().dimensions::<u32>();
    panel.set_cursor_x(point_x - cursor_width as f32 / 2.0);
    panel.set_cursor_y(point_y - cursor_height as f32 / 2.0);
  });
}

fn set_ui(app_window: Weak<AppWindow>, images_ref: Rc<RefCell<ColorPanelImages>>) {
  let app = app_window.upgrade().unwrap();
  let panel = ColorPanel::get(&app);
  let images = images_ref.clone();
  color_box(images_ref.clone(), Color::red(), Point::new(0, 0));
  panel.set_hue_bar(as_ui_image(&images.borrow().hue_bar.borrow()));
  panel.set_color_box_background(as_ui_image(&images.borrow().box_background.borrow()));
  panel.set_color_box_foreground(as_ui_image(&images.borrow().box_foreground.borrow()));
  panel.set_cursor(as_ui_image(&images.borrow().white_circle.borrow()));
}

fn set_cursor_color(app_window: Weak<AppWindow>, images_ref: Rc<RefCell<ColorPanelImages>>, color: Color) {
  let app = app_window.upgrade().unwrap();
  let panel = ColorPanel::get(&app);
  let images = images_ref.clone();
  let is_light_area = is_light_area(color.clone());
  // println!("Is light area: {}; color: {}", is_light_area, color.clone());
  let imgs = images.borrow();
  let cursor = if is_light_area {
    println!("Black circle");
    imgs.black_circle.borrow()
  } else {
    println!("White circle");
    imgs.white_circle.borrow()
  };
  panel.set_cursor(as_ui_image(&cursor));
}

fn point_to_color(hue: f32, x: f32, y: f32) -> Color {
  Color::from_hsv(360.0 - (hue * 360.0), x, 1.0 - y)
}

fn is_light_area(color: Color) -> bool {
  color.contrast_ratio(Color::black()) > color.contrast_ratio(Color::white())
}

fn color_box(images: Rc<RefCell<ColorPanelImages>>, color: Color, point: Point) {
  let clone = images.clone();
  let images_ref = clone.borrow_mut();
  let (width, _) = images_ref.box_background.borrow().dimensions::<u32>();

  let gradient = Gradient::from_to(Color::white(), color.clone());
  let path = Path::line((0.0, 0.0), (width as f32, 0.0));
  linear_gradient(&mut images_ref.box_background.borrow_mut(), path, gradient);
}

fn hue_selector(point: Point, direction: Direction) -> Image {
  let mut image = Image::new(HUE_BAR_WIDTH, HUE_BAR_HEIGHT);
  let (width, height) = image.dimensions::<u32>();

  let hue_colors = Gradient::hue();
  let path = if direction == Direction::Horizontal {
    Path::line((0.0, 0.0), (width as f32, 0.0))
  } else {
    Path::line((0.0, 0.0), (0.0, height as f32))
  };

  linear_gradient(&mut image, path, hue_colors);
  image
}

fn trans_to_black_gradient(width: u32, height: u32) -> Image {
  let mut image = Image::new(width, width);
  let gradient = Gradient::from_to(Color::transparent(), Color::black());
  let path = Path::line((0.0, 0.0), (0.0, height as f32));
  linear_gradient(&mut image, path, gradient);
  image
}

fn circle_image(radius: u32, color: Color) -> Image {
  let mut image = Image::new(radius * 2 + 2, radius * 2 + 2);
  image.clear_color(Color::transparent());
  let center = Point::new(radius as i32, radius as i32);

  // Create a circular path using multiple line segments
  let segments = 64; // More segments = smoother circle
  let mut path = Path::new();
  let mut first: Option<(f32, f32)> = None;
  for i in 0..=segments {
    let angle = (i as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
    let x = angle.cos() * radius as f32;
    let y = angle.sin() * radius as f32;
    if i == 0 {
      path.with_move_to((x, y));
      first = Some((x, y));
    } else {
      path.with_line_to((x, y));
    }
  }
  if let Some((fx, fy)) = first {
    path.with_line_to((fx, fy));
  }

  line::line(&mut image, center, path, abra::color::Fill::Solid(color), 1, None, None);
  image
}
