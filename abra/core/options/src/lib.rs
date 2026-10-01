//! Options and traits for applying effects. They live in `abra_core::image::effect`; this crate re-exports them so
//! effects and callers can keep importing from `options`.

pub use abra_core::image::effect::{Effect, ApplyOptions, ApplyTarget, Options, get_ctx};
pub use abra_core::image::gpu::Hardware;
