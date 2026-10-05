use std::sync::{Arc, Mutex};

use vessel_api::prelude::*;

use crate::Image;

/// A theme/color fill or an image painted behind content, independently of child layout and input.
///
/// ```
/// use vessel_api::prelude::*;
/// use vessel_ui::{Button, Container, Fit, Image};
///
/// let image = Image::new().with_fit(Fit::Cover);
/// let panel = Container::new().with_background(&image).with_display(Display::Flex);
/// let button = Button::new("Open").with_background(&image).with_text_color([255, 255, 255, 255]);
/// panel.add(&button);
/// button.set_background(Background::Control); // replace the image with a themed fill
/// ```
#[derive(Clone)]
pub enum UiBackground {
  /// A theme-based or explicit color background.
  Color(Background),
  /// An image fitted to the border box using its [`crate::Fit`], centered and clipped to rounded corners.
  /// This is a single, non-repeating image; GPU pictures are read back once per publication for CPU composition.
  Image(Image),
}

impl From<Background> for UiBackground {
  fn from(p_background: Background) -> Self {
    Self::Color(p_background)
  }
}

impl From<Image> for UiBackground {
  fn from(p_image: Image) -> Self {
    Self::Image(p_image)
  }
}

impl From<&Image> for UiBackground {
  fn from(p_image: &Image) -> Self {
    Self::Image(p_image.clone())
  }
}

#[derive(Clone)]
pub(crate) struct BackgroundLayer {
  revision: BehaviorSubject<u64>,
  image: Arc<Mutex<Option<Image>>>,
}

impl BackgroundLayer {
  pub fn new(p_component: &Component) -> Self {
    let layer = Self {
      revision: BehaviorSubject::new(0),
      image: Arc::default(),
    };
    p_component.subject::<Canvas>().subscribe({
      let layer = layer.clone();
      move |canvas| {
        layer.revision.value();
        let image = layer.image.lock().unwrap().clone();
        if let Some(image) = image {
          image.draw_background(canvas);
        }
      }
    });
    layer
  }

  pub fn set(&self, p_component: &Component, p_background: UiBackground) {
    match p_background {
      UiBackground::Color(color) => {
        *self.image.lock().unwrap() = None;
        p_component.set_background(color);
      }
      UiBackground::Image(image) => {
        *self.image.lock().unwrap() = Some(image);
      }
    }
    self.revision.next(self.revision.peek().wrapping_add(1));
  }
}

#[cfg(test)]
mod tests {
  use std::any::Any;
  use std::time::Duration;

  use super::*;
  use crate::{Button, Clicked, Container, Fit};

  const RED: [u8; 4] = [200, 0, 0, 255];
  const BLUE: [u8; 4] = [0, 0, 200, 255];

  #[test]
  fn child_updates_restore_prepared_backgrounds_and_match_full_composition() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    for fit in [Fit::Contain, Fit::Cover, Fit::Stretch] {
      let image = Image::new().with_fit(fit).with_pixels(2, 1, [RED, [0, 0, 200, 127]].concat());
      let container = Container::new()
        .with_background(&image)
        .with_radius(Units::Percent(35.0))
        .with_border_width(1)
        .with_border_color([255; 4]);
      container.size().next((24, 17));
      let own_draws = Arc::new(AtomicUsize::new(0));
      container.subject::<Canvas>().subscribe({
        let own_draws = Arc::clone(&own_draws);
        move |_| {
          own_draws.fetch_add(1, Ordering::SeqCst);
        }
      });
      let color = BehaviorSubject::new([120, 40, 20, 127]);
      let child = Component::new("changing").with_width(7).with_height(9);
      child.subject::<Canvas>().subscribe({
        let color = color.clone();
        move |canvas| canvas.fill(color.value())
      });
      let overlay = Component::new("overlap").with_width(4).with_height(12).with_margin([0, 3]);
      overlay.subject::<Canvas>().subscribe(|canvas| canvas.fill([0, 170, 0, 127]));
      container.add(&child).add(&overlay);
      let first = render(&container);

      for next in [[180, 80, 0, 255], [0; 4], [0, 0, 180, 80], [0; 4]] {
        let before = own_draws.load(Ordering::SeqCst);
        color.next(next);
        let partial = render(&container);
        assert_eq!(
          own_draws.load(Ordering::SeqCst),
          before,
          "child-only updates must reuse the parent's prepared layer"
        );
        assert_eq!(pixel(&partial, 20, 14), pixel(&first, 20, 14), "pixels outside the dirty area remain intact");
        container.set_border_color([255; 4]); // force the reference through full composition
        assert_eq!(partial.pixels, render(&container).pixels, "fit={fit:?}, child={next:?}");
      }

      // Reusing the layer must retain its reactive dependencies, and replacing it must forget the old source.
      color.next([1, 2, 3, 4]);
      render(&container);
      image.set_pixels(1, 1, BLUE.to_vec());
      assert!(container.component().clone().has_changed());
      let changed = render(&container);
      assert_eq!(pixel(&changed, 17, 8), BLUE);
      container.set_background(Image::new().with_fit(Fit::Stretch).with_pixels(1, 1, RED.to_vec()));
      render(&container);
      image.set_pixels(1, 1, RED.to_vec());
      assert!(!container.component().clone().has_changed());
    }
  }

  struct CountedGpuImage {
    reads: std::sync::atomic::AtomicUsize,
    color: Mutex<[u8; 4]>,
  }

  impl GpuFrame for CountedGpuImage {
    fn size(&self) -> (u32, u32) {
      (2, 1)
    }

    fn as_any(&self) -> &dyn Any {
      self
    }

    fn read_back(&self) -> Option<Frame> {
      self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
      Some(Frame {
        width: 2,
        height: 1,
        pixels: Arc::new(self.color.lock().unwrap().repeat(2)),
      })
    }
  }

  #[test]
  fn gpu_backgrounds_read_each_publication_once_across_foreground_style_fit_and_size_changes() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let gpu = Arc::new(CountedGpuImage {
      reads: AtomicUsize::new(0),
      color: Mutex::new(RED),
    });
    let image = Image::new().with_fit(Fit::Stretch);
    image.show_gpu(gpu.clone());
    let button = Button::new("A").with_background(&image).with_radius(0).with_border_width(0);
    button.size().next((24, 17));
    assert_eq!(pixel(&render(&button), 20, 8), RED);
    assert_eq!(gpu.reads.load(Ordering::SeqCst), 1);
    button.set_label("B");
    render(&button);
    button.set_radius(3);
    render(&button);
    button.set_theme(Theme::dark());
    render(&button);
    image.set_fit(Fit::Cover);
    render(&button);
    button.size().next((32, 24));
    render(&button);
    assert_eq!(gpu.reads.load(Ordering::SeqCst), 1, "style, fit and size reuse the readback");

    // Publishing the same GPU handle again is still a new picture: never key readbacks solely by its address.
    *gpu.color.lock().unwrap() = BLUE;
    image.show_gpu(gpu.clone());
    assert_eq!(pixel(&render(&button), 28, 12), BLUE);
    assert_eq!(gpu.reads.load(Ordering::SeqCst), 2);
    image.clear();
    render(&button);
    image.show_gpu(gpu.clone());
    render(&button);
    assert_eq!(gpu.reads.load(Ordering::SeqCst), 3);
    image.set_pixels(1, 1, RED.to_vec());
    assert_eq!(pixel(&render(&button), 28, 12), RED);
  }

  fn render(p_component: &impl Renderable) -> Frame {
    p_component.component().clone().render(Duration::ZERO)
  }

  fn pixel(p_frame: &Frame, p_x: u32, p_y: u32) -> [u8; 4] {
    let at = ((p_y * p_frame.width + p_x) * 4) as usize;
    p_frame.pixels[at..at + 4].try_into().unwrap()
  }

  fn image() -> Image {
    Image::new().with_pixels(2, 1, [RED, BLUE].concat())
  }

  #[test]
  fn backgrounds_follow_fit_live_pixels_and_parent_resizing() {
    let image = image();
    let container = Container::new().with_background(&image).with_radius(0).with_display(Display::Flex);
    container.size().next((4, 4));
    let frame = render(&container);
    assert_eq!(pixel(&frame, 0, 0), [0; 4], "contain leaves transparent bands");
    assert_eq!(pixel(&frame, 0, 1), RED);
    assert_eq!(pixel(&frame, 3, 1), BLUE);
    assert_eq!(image.size().value(), (4, 4));

    image.set_fit(Fit::Cover);
    assert!(container.component().clone().has_changed());
    let frame = render(&container);
    assert_eq!(pixel(&frame, 0, 0), RED);
    assert_eq!(pixel(&frame, 3, 3), BLUE);

    image.set_fit(Fit::Stretch);
    container.size().next((8, 2));
    let frame = render(&container);
    assert_eq!(image.size().value(), (8, 2));
    assert_eq!(pixel(&frame, 0, 0), RED);
    assert_eq!(pixel(&frame, 7, 1), BLUE);

    image.set_pixels(1, 1, BLUE.to_vec());
    assert!(container.component().clone().has_changed());
    assert_eq!(pixel(&render(&container), 0, 0), BLUE);
  }

  #[test]
  fn backgrounds_do_not_take_layout_space_or_input_and_are_below_children_and_borders() {
    let image = image().with_fit(Fit::Stretch);
    let touches = image.subject::<PointerEvent>().scan(0, |count, _| count + 1);
    let child = Component::new("child");
    child.subject::<Canvas>().subscribe(|canvas| canvas.fill([0, 200, 0, 255]));
    let container = Container::new().with_background(&image).with_radius(0).with_display(Display::Flex).add(&child);
    container.set_padding(2);
    container.set_border_width(1);
    container.set_border_color([255, 255, 255, 255]);
    container.size().next((10, 10));
    let frame = render(&container);
    assert_eq!(child.size().value(), (4, 4), "the image is not a flex child");
    assert_eq!(pixel(&frame, 1, 1), RED, "background covers padding");
    assert_eq!(pixel(&frame, 3, 3), [0, 200, 0, 255], "children are in front");
    assert_eq!(pixel(&frame, 0, 0), [255; 4], "border is in front");
    container.next(PointerEvent::Moved { x: 1.0, y: 1.0 });
    assert_eq!(touches.value(), 0);
  }

  #[test]
  fn replacement_clearing_and_theme_fills_work_and_old_images_stop_invalidating() {
    let image = image().with_fit(Fit::Stretch);
    let container = Container::new().with_background(&image).with_radius(0);
    container.size().next((4, 4));
    render(&container);
    container.set_background(Background::Surface);
    assert_eq!(pixel(&render(&container), 0, 0), Theme::light().surface);
    image.set_pixels(1, 1, BLUE.to_vec());
    assert!(!container.component().clone().has_changed());
    container.set_theme(Theme::dark());
    assert_eq!(pixel(&render(&container), 0, 0), Theme::dark().surface);
    container.set_background(Background::None);
    assert_eq!(pixel(&render(&container), 0, 0), [0; 4]);
    container.set_background(image);
    assert_eq!(pixel(&render(&container), 0, 0), BLUE);
  }

  #[test]
  fn button_backgrounds_preserve_rounding_label_clicks_and_pressed_feedback() {
    let image = image().with_fit(Fit::Stretch);
    let button = Button::new("OK").with_background(&image).with_text_color([0, 255, 0, 255]);
    button.size().next((80, 30));
    let clicks = button.subject::<Clicked>().scan(0, |count, _| count + 1);
    let frame = render(&button);
    assert_eq!(pixel(&frame, 0, 0)[3], 0);
    assert_eq!(pixel(&frame, 10, 5), RED);
    assert!(frame.pixels.chunks_exact(4).any(|pixel| pixel == [0, 255, 0, 255]));
    button.next(PointerEvent::Moved { x: 10.0, y: 5.0 });
    button.next(PointerEvent::Button {
      button: PointerButton::Left,
      pressed: true,
    });
    assert!(pixel(&render(&button), 10, 5)[0] < RED[0]);
    button.next(PointerEvent::Button {
      button: PointerButton::Left,
      pressed: false,
    });
    assert_eq!(clicks.value(), 1);
    button.set_color(BLUE);
    assert_eq!(pixel(&render(&button), 10, 5), BLUE);
    button.set_background(&image);
    button.set_primary(true);
    assert_eq!(pixel(&render(&button), 10, 5), Theme::light().accent);
  }

  struct GpuImage;

  impl GpuFrame for GpuImage {
    fn size(&self) -> (u32, u32) {
      (1, 1)
    }

    fn as_any(&self) -> &dyn Any {
      self
    }

    fn read_back(&self) -> Option<Frame> {
      Some(Frame {
        width: 1,
        height: 1,
        pixels: Arc::new(RED.to_vec()),
      })
    }
  }

  #[test]
  fn gpu_backgrounds_are_composed_and_can_switch_back_to_pixels() {
    let image = Image::new().with_fit(Fit::Stretch);
    let container = Container::new().with_background(&image).with_radius(0);
    container.size().next((4, 4));
    render(&container);
    image.show_gpu(Arc::new(GpuImage));
    assert!(container.component().clone().has_changed());
    assert_eq!(pixel(&render(&container), 0, 0), RED);
    image.set_pixels(1, 1, BLUE.to_vec());
    assert_eq!(pixel(&render(&container), 0, 0), BLUE);
    image.clear();
    assert_eq!(pixel(&render(&container), 0, 0), [0; 4]);
  }

  struct LiveImage;

  impl crate::Trackable for LiveImage {
    fn pictures(&self, p_size: &BehaviorSubject<(u32, u32)>) -> Subject<crate::Picture> {
      p_size.map(|&(width, height)| {
        crate::Picture::Pixels(Arc::new(Frame {
          width,
          height,
          pixels: Arc::new(RED.repeat(width as usize * height as usize)),
        }))
      })
    }
  }

  #[test]
  fn tracked_background_sources_receive_the_full_box_size_and_resize() {
    let image = Image::new().track(&LiveImage);
    let container = Container::new().with_background(image).with_radius(0);
    for size in [(4, 2), (6, 3)] {
      container.size().next(size);
      let frame = render(&container);
      assert_eq!(pixel(&frame, size.0 - 1, size.1 - 1), RED);
    }
  }

  #[test]
  fn percentage_radius_matches_pixel_rounding_for_image_backgrounds_borders_and_button_feedback() {
    let image = image().with_fit(Fit::Stretch);
    let container = Container::new().with_background(&image).with_radius(Units::Percent(50.0)).with_border_width(1);
    let reference = Container::new().with_background(&image).with_radius(Units::Pixels(10)).with_border_width(1);
    for size in [(40, 20), (20, 40), (20, 20)] {
      container.size().next(size);
      reference.size().next(size);
      assert_eq!(render(&container).pixels, render(&reference).pixels);
      assert_eq!(pixel(&render(&container), 0, 0)[3], 0);
    }
    let button = Button::new("").with_background(&image).with_radius(Units::Percent(25.0));
    let reference = Button::new("").with_background(&image).with_radius(Units::Pixels(5));
    button.size().next((40, 20));
    reference.size().next((40, 20));
    assert_eq!(render(&button).pixels, render(&reference).pixels);
    for control in [&button, &reference] {
      control.next(PointerEvent::Button {
        button: PointerButton::Left,
        pressed: true,
      });
    }
    assert_eq!(render(&button).pixels, render(&reference).pixels);
  }

  struct UnreadableImage;

  impl GpuFrame for UnreadableImage {
    fn size(&self) -> (u32, u32) {
      (1, 1)
    }

    fn as_any(&self) -> &dyn Any {
      self
    }
  }

  #[test]
  #[should_panic(expected = "background GPU image could not be read back")]
  fn unreadable_gpu_backgrounds_report_failure() {
    let image = Image::new();
    image.show_gpu(Arc::new(UnreadableImage));
    let container = Container::new().with_background(image);
    container.size().next((1, 1));
    render(&container);
  }
}
