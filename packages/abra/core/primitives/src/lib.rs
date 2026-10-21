//! Minimal primitives crate containing the core Image type and small supporting types.
//! This crate is intended to be light-weight and free of heavy dependencies such as IO and transforms.

pub mod channels;
pub mod color;
pub mod gaussian;
pub mod image;
pub mod resolution;

pub use self::channels::{Channel, ChannelMask, Channels};
pub use self::color::{
  Bins, Color, ColorSpace, ColorStat, Harmony, Histogram, HueDirection, ColorInterpolation, LumaStandard, Palette,
  luma,
};
pub use self::image::{DeferredPixels, Image};
pub use self::resolution::Resolution;
