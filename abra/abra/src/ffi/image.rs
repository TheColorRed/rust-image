use abra_core::{Color, Image};

constructor_ffi!(abra_image_new, Image, Image::new, width: u32, height: u32);

#[unsafe(no_mangle)]
pub extern "C" fn abra_image_new_from_color(p_width: u32, p_height: u32, p_color: *const Color) -> *mut Image {
  let p_color = if p_color.is_null() { Color::black() } else { unsafe { *p_color } };
  box_ffi!(Image::new_from_color(p_width, p_height, p_color))
}

destructor_ffi!(abra_image_free, Image);
