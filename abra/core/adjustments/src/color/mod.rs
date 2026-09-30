mod auto_color;
mod auto_tone;
mod gradient_map;
mod grayscale;
mod invert;
mod linear_gradient;
mod opacity;
mod posterize;
mod threshold;

pub use auto_color::{AutoColor, auto_color};
pub use auto_tone::{AutoTone, auto_tone};
pub use gradient_map::{GradientMap, gradient_map};
pub use grayscale::{Grayscale, grayscale};
pub use invert::{Invert, invert};
pub use linear_gradient::LinearGradientEffect;
pub use opacity::{Opacity, reduce_opacity};
pub use posterize::{Posterize, posterize};
pub use threshold::{Threshold, threshold};
