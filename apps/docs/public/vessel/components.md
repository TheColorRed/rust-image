---
title: Building components
order: 1
outline: deep
---

# Building components

An app assembles `Component`s and reusable controls into a tree. `Component` is the common base: it draws, exposes typed event streams, and lays out children. Ready-made controls such as `Button`, `Label`, `Slider`, `Image`, and `Container` are components too.

## Compose a layout

`Stack` is the default layout. Use `Flex` for a row or column and `Grid` for tracks. The container determines the size of its children; set a child's dimensions with methods such as `.width(..)`, `.height(..)`, `.fill()`, `.grow(..)`, or `.span(..)`.

```rust
use vessel::prelude::*;

fn main() {
  let save = Button::new("Save").with_primary(true);
  save.subject::<Clicked>().subscribe(|_| println!("Saved"));

  let panel = Component::new("panel")
    .with_display(Display::Flex)
    .with_direction(Direction::Column)
    .with_gap(8)
    .with_padding(8)
    .with_background(Background::Surface)
    .width(220);

  panel
    .add(Label::new("Editor").height(32))
    .add(save.height(48));

  let app = App::new("Editor")
    .with_size(960, 540)
    .with_quit_key("Escape")
    .add(&panel);
  app.run().expect("the window failed");
}
```

Layout, theme, and font values use `with_*` builders and `set_*` setters. Children do not need platform-specific code. For window apps, keyboard input goes to the focused component and pointer input goes to the component under the pointer.

## Rendering and background caching

When only a child changes and the layout stays the same, Vessel restores the affected region from the parent's cached drawing, then composites the children that overlap it and repaints the border. The parent's drawing listeners do not run again. Image fitting, rounded backgrounds, and other parent drawing are reused; the reactive properties read by the last draw still invalidate that cache when they change.

Parents retain one additional RGBA layer before children and borders. Leaves use their final frame without an additional layer. Size, layout, styling, and background-source changes rebuild the parent layer. Frames already held by a consumer remain unchanged.

Image backgrounds cache fitted CPU pixels by source, size, and `Fit`. A GPU background is read back once per published picture, and those pixels are reused across text, styling, fit, and size changes. Publishing a picture with `show_gpu` invalidates its readback even if the GPU handle is reused; replacing or clearing a picture releases its caches. Background composition remains on the CPU, while standalone GPU images can still be presented directly.

Background painting copies opaque interior spans and blends translucent pixels with the same alpha rounding as before. Rounded-corner coverage is calculated only in corner regions, and painting respects the dirty-region clip.

## React to events and state

Events are typed streams. Subscribe to a component's event type; use `next` to send an event. A `Slider` emits `Changed(f32)`, and a `Button` emits `Clicked`.

Components can read another component's current value directly. This example reads the slider value while drawing, so the preview redraws when that value changes:

```rust
use vessel::prelude::*;

fn make_preview() -> (Slider, Component) {
  let slider = Slider::new(0.5);
  let brightness = slider.clone();

  let preview = Component::new("preview");
  preview.subject::<Canvas>().subscribe(move |canvas| {
    let level = 0.1 + brightness.value() * 1.4;
    canvas.fill_with(|x, y| [x * level, y * level, 0.5 * level]);
  });

  let page = Component::new("page")
    .with_display(Display::Flex)
    .with_gap(12);
  page.add(&slider).add(&preview);
  (slider, page)
}
```

The `Canvas` event supplies a drawing handle. Values read during drawing are tracked automatically; a change redraws the component without an explicit invalidation call. For a value that is not read during drawing, use `watch` to connect it to redraws.

## Wrap a source in a component

The `Component` derive lets an app expose one of its renderable fields as a component. The mobile app's thumbnail follows an image source and places a label over it:

```rust
use std::sync::Arc;

use crate::AbraImage;
use vessel::prelude::*;

#[derive(uniffi::Object, Component)]
pub struct ThumbnailPreview {
  container: Container,
}

#[uniffi::export]
impl ThumbnailPreview {
  #[uniffi::constructor]
  pub fn new(image_source: Arc<AbraImage>, label: String) -> Arc<Self> {
    let image = Image::new()
      .with_fit(Fit::Cover)
      .track(&image_source);
    let text = Label::new(label);

    let container = Container::new()
      .with_background(&image)
      .with_radius(8)
      .with_border_color([255, 255, 255, 255])
      .with_border_width(2);
    container.add(&text);

    Arc::new(Self { container })
  }

  pub fn set_image(&self, image_source: Arc<AbraImage>) {
    let image = Image::new()
      .with_fit(Fit::Cover)
      .track(&image_source);
    self.container.set_background(image);
  }
}
```

`AbraImage` here is the app's own source type; it implements Vessel's `Trackable` interface. Keep adapters that use Abra in the app rather than adding Abra as a dependency of Vessel. `Image::track` follows later source changes, and replacing the background updates the existing thumbnail without remounting it.

## Connect sibling components with typed messages

A component can subscribe to a child's stream and send a typed message to another component. The mobile skin-tone picker uses a `Slider` with custom track and thumb images, then forwards every drag position to its preview:

```rust
use crate::components::image_preview::ImagePreview;
use std::sync::Arc;
use vessel::prelude::*;

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum Message {
  SliderMove(String, f64),
}

fn connect_slider(
  preview: &ImagePreview,
  start_value: f64,
  gradient_frame: Frame,
  handle_frame: Frame,
) -> Slider {
  let slider = Slider::new(start_value as f32)
    .with_track_image(Arc::new(gradient_frame))
    .with_thumb_image(Arc::new(handle_frame));

  let preview_messages = preview.subject::<Message>();
  slider.changes().subscribe(move |changed| {
    preview_messages.next(Message::SliderMove(
      "action-skin-tan".into(),
      changed.0 as f64,
    ));
  });
  slider
}
```

The preview receives only the changed value and decides what to render. For a value to apply when a drag ends, read `slider.value()` at that point. Calling `slider.set_value(..)` from the program moves the handle but does not send a preview message.

## Learn from the in-repository examples

- `apps/examples/vessel-panels` demonstrates a complete desktop window app and reactive drawing.
- `packages/react-native/alakazam/src/components/thumbnail_preview.rs` demonstrates a tracked image with an overlaid label and reactive replacement.
- `packages/react-native/alakazam/src/components/skin_tan_color_picker.rs` demonstrates custom slider images and forwarding typed messages.
