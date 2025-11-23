use core::Color;

pub fn test() {
  Color::mean(vec![0u8, 255, 128].as_slice());
}

constructor_ffi!(abra_color_rgb, Color, Color::from_rgb, r: u8, g: u8, b: u8);
constructor_ffi!(abra_color_rgba, Color, Color::from_rgba, r: u8, g: u8, b: u8, a: u8);
constructor_ffi!(abra_color_hex, Color, Color::from_hex, hex: u32);
constructor_ffi!(abra_color_hsl, Color, Color::from_hsl, h: f32, s: f32, l: f32);
constructor_ffi!(abra_color_hsv, Color, Color::from_hsv, h: f32, s: f32, v: f32);
// Accepts pointer + len for an array/slice of u8 colors to be combined
constructor_ffi_slice!(abra_color_average, Color, Color::average, c: *const u8, len: usize);
constructor_ffi_slice!(abra_color_mean, Color, Color::mean, c: *const u8, len: usize);
constructor_ffi_slice!(abra_color_median, Color, Color::median, c: *const u8, len: usize);
constructor_ffi_slice!(abra_color_mode, Color, Color::mode, c: *const u8, len: usize);

destructor_ffi!(abra_color_free, Color);

// Include the generated color_list! mappings produced by build.rs. The build
// script parses the core color definitions and emits a color_list! invocation
// that maps abra_color_... -> <color_fn> for every zero-arg color constructor.
// File contains: `color_list!( ... );`
include!(concat!(env!("OUT_DIR"), "/generated_color_list.rs"));
