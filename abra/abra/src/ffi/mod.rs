// Box FFI values using macro
// Accept an expression and return a raw pointer for FFI.
macro_rules! box_ffi {
  ($value:expr) => {
    Box::into_raw(Box::new($value))
  };
}

macro_rules! unbox_ffi {
  ($value:expr) => {
    if !$value.is_null() {
      unsafe {
        drop(Box::from_raw($value));
      }
    }
  };
}

// Convenience helpers for generating FFI wrappers for constructors/destructors
// These macros reduce the repetition of declaring `#[unsafe(no_mangle)] pub extern "C" fn ...` wrappers.
// Usage examples:
//   constructor_ffi!(abra_color_rgb, Color, Color::from_rgb, r: u8, g: u8, b: u8);
//   destructor_ffi!(abra_color_free, Color);
macro_rules! constructor_ffi {
  ($fn_name:ident, $ty:ty, $ctor:path $(, $arg_name:ident : $arg_ty:ty )* ) => {
    #[unsafe(no_mangle)]
    pub extern "C" fn $fn_name( $( $arg_name : $arg_ty ),* ) -> *mut $ty {
      box_ffi!($ctor( $( $arg_name ),* ))
    }
  };
}

macro_rules! destructor_ffi {
  ($fn_name:ident, $ty:ty) => {
    #[unsafe(no_mangle)]
    pub extern "C" fn $fn_name(ptr: *mut $ty) {
      unbox_ffi!(ptr);
    }
  };
}

// Convenience helper for slice arguments: accept pointer + length and covert to
// an `&[T]` before calling the inner constructor.
// Usage: constructor_ffi_slice!(abra_color_mean, Color, Color::mean, data: *const u8, len: usize);
macro_rules! constructor_ffi_slice {
  ($fn_name:ident, $ty:ty, $ctor:path, $ptr_name:ident : *const $elem_ty:ty, $len_name:ident : usize) => {
    #[unsafe(no_mangle)]
    pub extern "C" fn $fn_name($ptr_name: *const $elem_ty, $len_name: usize) -> *mut $ty {
      let slice: &[$elem_ty] = if $ptr_name.is_null() {
        &[]
      } else {
        unsafe { std::slice::from_raw_parts($ptr_name, $len_name) }
      };
      box_ffi!($ctor(slice))
    }
  };
}

pub mod color;
pub mod image;
