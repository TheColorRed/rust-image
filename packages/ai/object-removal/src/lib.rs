//! Object removal plugins: fill the part of a photo a mask covers with what belongs there.
//!
//! There is no code in this crate to run them. Each folder in `plugins` is a runtime plugin (see `packages/react-native/alakazam/src/plugins`) that adds an AI model
//! with the `inpaint` capability, described entirely by its manifest, and `AiModel` runs it like any other model:
//! `AiModel::load(path)?.run_guided(&photo, Some(&mask), &|_| {})` gives `AiOutput::Image`, the size of the photo, with only the masked
//! pixels changed. See `abra_ai_core::graph` for what a manifest can say, and `plugins/README.md` for MI-GAN (a pipeline that crops and
//! pastes back itself) and LaMa (a fixed 512 x 512 square that Abra cuts round the mask).
//!
//! What lives here is the plugins' files, and the tests that check every plugin with the `inpaint` capability does what it says. A new
//! remover is a new folder in `plugins`, and these tests run it without being changed.

pub mod prelude {
  pub use abra_ai_core::{AiError, AiModel, AiOutput};
  pub use mask::Mask;
}
