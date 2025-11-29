pub mod apply_area;
pub mod gpu_op;
pub mod gpu_registry;
pub mod image_ext;

// Re-export the primitives Image type at `abra_core::Image` so existing imports continue to work.
pub use primitives::Image;
// Re-export the ext trait so callers can call open()/save() on `abra_core::Image` if they `use abra_core::prelude::*` or import the trait
// pub use image_ext::image_area::*;
// pub use image_ext::image_ext::*;
// pub use image_ext::image_size::*;
// Helper free functions wrapping CoreImageExt methods
