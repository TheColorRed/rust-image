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

/// A set of color channels, such as the ones an operation is allowed to change. Every channel is in the set by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelMask {
  bits: u8,
}

impl Default for ChannelMask {
  fn default() -> Self {
    ChannelMask::ALL
  }
}

impl ChannelMask {
  /// Red, green, blue and alpha.
  pub const ALL: ChannelMask = ChannelMask { bits: 0b1111 };

  /// No channels.
  pub const NONE: ChannelMask = ChannelMask { bits: 0 };

  /// Red, green and blue without alpha.
  pub const RGB: ChannelMask = ChannelMask { bits: 0b0111 };

  /// A set of just the given channels.
  pub fn of(p_channels: impl IntoIterator<Item = Channel>) -> Self {
    p_channels.into_iter().fold(ChannelMask::NONE, |mask, channel| mask.with(channel, true))
  }

  /// Whether the channel is in the set.
  pub const fn contains(&self, p_channel: Channel) -> bool {
    self.bits & (1 << p_channel.index()) != 0
  }

  /// The set with the channel added or removed.
  pub const fn with(self, p_channel: Channel, p_enabled: bool) -> Self {
    let bit = 1 << p_channel.index();
    ChannelMask { bits: if p_enabled { self.bits | bit } else { self.bits & !bit } }
  }

  /// Whether every channel is in the set.
  pub const fn is_all(&self) -> bool {
    self.bits == ChannelMask::ALL.bits
  }

  /// Takes the channels in the set from `p_new` and the rest from `p_old`.
  pub fn select(&self, p_old: [u8; 4], p_new: [u8; 4]) -> [u8; 4] {
    let pick = |channel: Channel| if self.contains(channel) { p_new[channel.index()] } else { p_old[channel.index()] };
    [pick(Channel::R), pick(Channel::G), pick(Channel::B), pick(Channel::A)]
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
