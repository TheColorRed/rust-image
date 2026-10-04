//! A standalone Vessel app on a desktop window: a gradient whose brightness follows the Up and Down arrow keys.
//!
//! It uses no abra and no React Native, only the API crate and the `desktop-window` flag. Nothing in it names a
//! platform: the component only sees its size and the events its host sends it.
//!
//! ```text
//! cargo run -p vessel-window-test --bin vessel-window                 # a normal window
//! cargo run -p vessel-window-test --bin vessel-window -- --borderless # no title bar or border
//! cargo run -p vessel-window-test --bin vessel-window -- --fullscreen # fullscreen without borders
//! cargo run -p vessel-window-test --bin vessel-window -- --exit-after 5
//! ```
//!
//! Up and Down change the brightness, and Escape closes the window.

use std::time::Duration;

use vessel::prelude::*;

fn main() {
  let arguments: Vec<String> = std::env::args().collect();
  let has = |flag: &str| arguments.iter().any(|argument| argument == flag);
  let frame = if has("--fullscreen") {
    AppFrame::Fullscreen
  } else if has("--borderless") {
    AppFrame::Borderless
  } else {
    AppFrame::Bordered
  };
  let exit_after = arguments
    .iter()
    .position(|argument| argument == "--exit-after")
    .and_then(|at| arguments.get(at + 1))
    .and_then(|seconds| seconds.parse::<u64>().ok());

  let gradient = Component::new("gradient");

  // Key presses become steps, and the steps fold into a running brightness: operators on the component's keyboard stream.
  let brightness = gradient
    .subject::<KeyEvent>()
    .filter_map(|event| match (event.pressed, event.key.as_str()) {
      (true, "ArrowUp") => Some(0.1_f32),
      (true, "ArrowDown") => Some(-0.1),
      _ => None,
    })
    .scan(0.8_f32, |brightness, step| (brightness + step).clamp(0.1, 1.5));

  // How it looks. It redraws whenever the brightness it reads changes.
  gradient.subject::<Canvas>().subscribe({
    let brightness = brightness.clone();
    move |canvas| {
      let brightness = brightness.value();
      canvas.fill_with(|x, y| [x * brightness, y * brightness, 0.5 * brightness]);
    }
  });

  // Escape asks the host to end the app, with the component's Quit event.
  let _escape = gradient.subject::<KeyEvent>().filter(|event| event.pressed && event.key == "Escape").subscribe({
    let gradient = gradient.clone();
    move |_| gradient.next(Quit)
  });

  if let Some(seconds) = exit_after {
    let gradient = gradient.clone();
    std::thread::spawn(move || {
      std::thread::sleep(Duration::from_secs(seconds));
      gradient.next(Quit);
    });
  }

  run(&gradient, AppOptions::new("Vessel window").with_size(960, 540).with_frame(frame)).expect("the window failed");
}
