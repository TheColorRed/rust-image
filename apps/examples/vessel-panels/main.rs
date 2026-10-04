//! A Vessel app made of ready-made components: a side panel of labels, two buttons and a slider, beside a gradient.
//!
//! - The top button swaps the gradient's red and green. The bottom one dims it.
//! - The slider sets the brightness. Escape ends the app.
//!
//! Nothing here sets a color or a font for the panel: it follows the system theme (dark or light, and the accent color),
//! like a web page follows the browser.
//!
//! ```text
//! cargo run -p vessel-panels-test --bin vessel-panels
//! ```

use vessel::prelude::*;

fn main() {
  // The parts. Their state is not stored anywhere else: a click flips a value, and the slider holds its own.
  let swap = Button::new("Swap colors").with_primary(true);
  let dim = Button::new("Dim");
  dim.set_radius(24.0);
  let slider = Slider::new(0.5);
  let swapped = swap.subject::<Clicked>().scan(false, |on, _| !*on);
  let dimmed = dim.subject::<Clicked>().scan(false, |on, _| !*on);

  // The gradient draws from them. Whatever it reads, it redraws for: there is nothing to declare.
  let level = slider.clone();
  let gradient = Component::new("gradient");
  gradient.subject::<Canvas>().subscribe(move |canvas| {
    let brightness = (0.1 + level.value() * 1.4) * if dimmed.value() { 0.5 } else { 1.0 };
    let swapped = swapped.value();
    canvas.fill_with(|x, y| {
      let (red, green) = if swapped { (y, x) } else { (x, y) };
      [red * brightness, green * brightness, 0.5 * brightness]
    });
  });

  // One kind of component, laid out by settings, like a div in HTML: a column beside the gradient.
  let panel = Component::new("panel")
    .with_display(Display::Flex)
    .with_direction(Direction::Column)
    .with_gap(8)
    .with_padding(8)
    .with_background(Background::Surface)
    .width(220);
  panel
    .add(Label::new("Vessel panels").height(32))
    .add(swap.height(48))
    .add(dim.height(48))
    .add(Label::new("Brightness").with_font_size(12).height(32))
    .add(slider.height(28));
  let page = Component::new("page").with_display(Display::Flex);
  page.add(&panel).add(&gradient);

  let app = App::new("Vessel panels").with_size(960, 540).with_quit_key("Escape").add(&page);

  // Up and Down move the slider, wherever the keyboard is.
  // app.root().subject::<KeyEvent>().subscribe(move |key| {
  //   let step = match (key.pressed, key.key.as_str()) {
  //     (true, "ArrowUp") => 0.1,
  //     (true, "ArrowDown") => -0.1,
  //     _ => return,
  //   };
  //   slider.set_value(slider.value() + step);
  // });

  app.run().expect("the window failed");
}
