/// The channel layout of a pixel buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channels {
  RGBA,
  RGB,
}

impl Channels {
  /// Number of bytes per pixel for this layout.
  pub const fn bytes_per_pixel(&self) -> usize {
    match self {
      Channels::RGBA => 4,
      Channels::RGB => 3,
    }
  }
}

/// A single color channel within an RGBA pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
  R,
  G,
  B,
  A,
}

impl Channel {
  /// The red, green, and blue channels (no alpha).
  pub const RGB: [Channel; 3] = [Channel::R, Channel::G, Channel::B];

  /// All four channels.
  pub const RGBA: [Channel; 4] = [Channel::R, Channel::G, Channel::B, Channel::A];

  /// Byte offset of this channel within an RGBA pixel.
  pub const fn index(&self) -> usize {
    match self {
      Channel::R => 0,
      Channel::G => 1,
      Channel::B => 2,
      Channel::A => 3,
    }
  }
}
