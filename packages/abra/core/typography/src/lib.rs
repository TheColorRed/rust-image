//! Font loading and glyph metrics for Abra typography.
//!
//! This module wraps the `swash` crate to provide font loading from files or
//! in-memory bytes, along with variable-font-aware glyph rasterization used
//! by the drawing pipeline to render text.
//!
//! Examples
//! ```ignore
//! use typography::Font;
//! let font = Font::from_path("assets/MyFont.ttf")?;
//! let text = font.text("Hello").with_size(32.0).with_weight(700);
//! ```

mod font;
mod font_loader;
mod text;

pub use font::Font;
pub use font_loader::{FontLoadMode, FontLoader, IntoFontArc, LoadedFonts, load_fonts};
pub use text::{
  Text, TextAlign, TextAppearance, TextContent, TextDecoration, TextJustify, TextLayout, TextOverflow, TextSize,
  TextStyle, WordWrap,
};
