//! Minimal primitives crate containing the core Image type and small supporting types.
//! This crate is intended to be light-weight and free of heavy dependencies such as IO and transforms.

// The scaffolding other languages bind to; the types that carry `cfg_attr(feature = "uniffi", ...)` export through it.
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

pub mod channels;
pub mod color;
pub mod image;
pub mod resolution;

pub use self::channels::{Channel, Channels};
pub use self::color::{Bins, Color, ColorStat, Harmony, Histogram, LumaStandard, luma};
pub use self::image::{DeferredPixels, Image};
pub use self::resolution::Resolution;
