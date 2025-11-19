mod gradient_map;
mod grayscale;
mod invert;
mod opacity;
mod posterize;
mod threshold;

pub use gradient_map::gradient_map;
pub use gradient_map::gradient_map_reverse;
pub use grayscale::grayscale;
pub use invert::invert;
pub use opacity::reduce_opacity;
pub use posterize::posterize;
pub use threshold::threshold;
