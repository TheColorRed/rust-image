//! The Abra image processing library.
//! It provides core functionalities for image manipulation
//! and a plugin system for extending its capabilities.

use abra_core::Settings;
use ctor::ctor;

pub mod ffi;
pub mod plugin;

pub use abra_core;

// Convenience prelude: re-export commonly used items to simplify consumer imports.
pub mod prelude;
pub mod adjustments {
  pub mod prelude {
    pub use ::adjustments::*;
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
pub mod transform {
  pub mod prelude {
    pub use abra_core::transform::*;
  }
}
#[cfg(feature = "gpu_integration")]
// Ensure the gpu_integration crate is linked (and its crate-init code runs) when
extern crate gpu_integration as _gpu_integration;

#[ctor]
fn init_abra_core() {
  init_settings();
  init_gpu_integration();
}

/// Initialize global settings for Abra.
fn init_settings() {
  Settings::init();
}

#[cfg(not(feature = "gpu"))]
fn init_gpu_integration() {
  // No-op when GPU feature is disabled
}

#[cfg(feature = "gpu")]
/// Initialize GPU integration on crate load if enabled in settings.
fn init_gpu_integration() {
  let is_gpu_enabled = Settings::gpu_enabled();
  if is_gpu_enabled {
    use crate::abra_core::image::gpu_registry::get_gpu_provider;
    use std::time::{Duration, Instant};
    let start = Instant::now();
    let timeout = Duration::from_millis(250);
    while get_gpu_provider().is_none() && start.elapsed() < timeout {
      std::thread::sleep(Duration::from_millis(10));
    }
    // Spawn a background thread to initialize GPU provider to avoid blocking crate
    // load. This mirrors the behavior of `gpu_integration`'s own ctor, but ensures
    // we trigger initialization here so the crate isn't optimized away by the
    // linker when unused.
    std::thread::spawn(|| {
      println!("abra: background thread starting gpu_integration init");
      match _gpu_integration::init_gpu_blocking() {
        Ok(_) => println!("abra: GPU provider registered via gpu_integration"),
        Err(e) => println!("abra: GPU provider init failed: {:?}", e),
      }
    });
  }
}
