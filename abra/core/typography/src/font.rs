use std::{path::Path, sync::Arc};

use swash::{CacheKey, FontRef};

/// A loaded font face used to query metrics and rasterize glyphs.
#[derive(Clone)]
pub struct Font {
  data: Arc<Vec<u8>>,
  offset: u32,
  key: CacheKey,
}

impl Font {
  /// Loads a font from the file at `p_path` (e.g. a `.ttf` or `.otf` file).
  pub fn from_path(p_path: impl AsRef<Path>) -> anyhow::Result<Self> {
    let bytes = std::fs::read(p_path)?;
    Self::from_bytes(bytes)
  }

  /// Loads a font from raw font file bytes (e.g. a `.ttf` or `.otf` file).
  ///
  /// If the font is a variable font (e.g. containing a `wght` axis), pass a
  /// [`Text`](crate::Text) built with [`with_weight`](crate::Text::with_weight)
  /// to render a specific weight instance.
  pub fn from_bytes(p_bytes: impl Into<Vec<u8>>) -> anyhow::Result<Self> {
    let data = Arc::new(p_bytes.into());
    let font_ref = FontRef::from_index(&data, 0).ok_or_else(|| anyhow::anyhow!("failed to parse font"))?;
    let offset = font_ref.offset;
    let key = font_ref.key;
    Ok(Self { data, offset, key })
  }

  /// Returns a transient reference to the underlying parsed font, used to access
  /// metrics, glyph outlines, and other font data.
  pub(crate) fn as_ref(&self) -> FontRef<'_> {
    FontRef {
      data: &self.data,
      offset: self.offset,
      key: self.key,
    }
  }

  /// Returns the original font bytes for crate-internal shaping.
  pub(crate) fn bytes(&self) -> &[u8] {
    &self.data
  }
}
