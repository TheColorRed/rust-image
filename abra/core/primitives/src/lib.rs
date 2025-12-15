//! Minimal primitives crate containing the core Image type and small supporting types.
//! This crate is intended to be light-weight and free of heavy dependencies such as IO and transforms.

pub mod channels;
pub mod color;
pub mod image;

pub use self::channels::Channels;
pub use self::color::Color;
pub use self::image::Image;
