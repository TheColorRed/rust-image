//! The Abra image processing library.
//! It provides core functionalities for image manipulation
//! and a plugin system for extending its capabilities.

use abra_core::Settings;
use ctor::ctor;

pub mod ffi;
pub mod live;
pub mod plugin;

pub use abra_core;

// Convenience prelude: re-export commonly used items to simplify consumer imports.
pub mod prelude;
pub mod adjustments {
  pub mod prelude {
    pub use ::adjustments::*;
  }
}
pub mod tools {
  pub mod prelude {
    pub use ::tools::*;
  }
}
pub mod canvas {
  pub mod prelude {
    pub use ::canvas::*;
  }
}
pub mod drawing {
  pub mod prelude {
    pub use ::drawing::*;
  }
}
pub mod filters {
  pub mod prelude {
    pub use ::filters::*;
  }
}
pub mod mask {
  pub mod prelude {
    pub use ::mask::*;
  }
}
pub mod options {
  pub mod prelude {
    pub use ::options::*;
  }
}
pub mod typography {
  pub mod prelude {
    pub use ::typography::*;
  }
}
pub mod transform {
  pub mod prelude {
    pub use abra_core::transform::*;
  }
}
#[ctor(unsafe)]
fn init_abra_core() {
  init_settings();
  #[cfg(feature = "gpu")]
  gpu::register();
}

/// Initialize global settings for Abra.
fn init_settings() {
  Settings::init();
}
