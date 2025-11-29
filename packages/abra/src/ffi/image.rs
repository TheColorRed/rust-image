use abra_core::{Color, Image};

constructor_ffi!(abra_image_new, Image, Image::new, width: u32, height: u32);

#[unsafe(no_mangle)]
pub extern "C" fn abra_image_new_from_color(width: u32, height: u32, color: *const Color) -> *mut Image {
  let color = if color.is_null() {
    Color::black()
  } else {
    unsafe { *color }
  };
  box_ffi!(Image::new_from_color(width, height, color))
}

destructor_ffi!(abra_image_free, Image);
