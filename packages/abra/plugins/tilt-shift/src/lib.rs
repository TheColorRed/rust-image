use abra::plugin::{Plugin, PluginError, PluginResult};

pub struct TiltShift;

impl<'a> Plugin<'a> for TiltShift {
  fn name(&self) -> &str {
    "Tilt Shift"
  }

  fn description(&self) -> &str {
    "Applies a tilt-shift effect to the image, simulating a miniature scene by blurring areas outside a defined focus region and increasing saturation and contrast."
  }

  fn apply(&mut self) -> Result<PluginResult<'a>, PluginError> {
    let start = std::time::Instant::now();
    let result = PluginResult::new();

    println!("TiltShiftPlugin applied in {:?}", start.elapsed());
    Ok(result)
  }
}
