//! The `Effect` trait of the library, for the handle of every effect.
//!
//! Every effect is applied the same way, by the functions of the library's `Effect` trait, so [`effect_methods!`] writes them
//! for each handle: `apply` on whichever hardware suits, `apply_on_cpu` and `apply_on_gpu`. A handle holds the library's effect
//! as its only field. The last function returns an error in the library when the effect has no shader or no GPU is there, and
//! here it panics with that message, as `Image::read` does. `Effect::with_mask` and `with_area` are exported, and `with_areas` sets the areas of the options; each
//! giving a changed copy of the effect (the handles do not change in place). `ApplyOptions::with_hardware` is not exported.
//!
//! A macro is needed because uniffi exports only the functions written in an `#[uniffi::export] impl` block: the default
//! functions of a trait, or an empty `impl Trait for Handle {}`, would export nothing.

/// Gives each handle the functions of the library's `Effect` trait. The handle is a tuple struct holding the library's effect.
///
/// ```ignore
/// #[derive(uniffi::Object)]
/// pub struct Posterize(color::Posterize);
/// effect_methods!(Posterize);
/// ```
macro_rules! effect_methods {
  // A handle whose effect is behind a lock, because the library changes it in place.
  (locked $handle:ident) => {
    #[uniffi::export]
    impl $handle {
      /// A copy of the effect limited to a mask: black is no effect, white is full effect, gray is partial. The mask must be
      /// the size of the image. It keeps the area and hardware the effect already had. Nothing removes the mask.
      pub fn with_mask(&self, mask: Option<::std::sync::Arc<$crate::Mask>>) -> ::std::sync::Arc<Self> {
        let effect = self.0.read().unwrap().clone();
        ::std::sync::Arc::new(Self(::std::sync::RwLock::new(::abra::options::prelude::Effect::with_mask(effect, mask.map(|mask| ::abra::abra_core::GrayPlane::from(mask.core()))))))
      }

      /// A copy of the effect limited to an area, which may be feathered. It keeps the mask and hardware the effect already
      /// had. Nothing removes the area.
      pub fn with_area(&self, area: Option<::std::sync::Arc<$crate::Area>>) -> ::std::sync::Arc<Self> {
        let effect = self.0.read().unwrap().clone();
        ::std::sync::Arc::new(Self(::std::sync::RwLock::new(::abra::options::prelude::Effect::with_area(effect, area.map(|area| area.core())))))
      }

      /// A copy of the effect limited to several areas.
      pub fn with_areas(&self, areas: Vec<::std::sync::Arc<$crate::Area>>) -> ::std::sync::Arc<Self> {
        let effect = self.0.read().unwrap().clone();
        let areas: Vec<_> = areas.iter().map(|area| area.core()).collect();
        let options = ::abra::options::prelude::Effect::options(&effect).clone().unwrap_or_default().with_areas(areas);
        ::std::sync::Arc::new(Self(::std::sync::RwLock::new(::abra::options::prelude::Effect::with_options(effect, options))))
      }

      /// Applies the effect to the image now, on whichever hardware suits.
      pub fn apply(&self, image: ::std::sync::Arc<$crate::Image>) {
        let effect = self.0.read().unwrap().clone();
        image.edit(|image| ::abra::options::prelude::Effect::apply(&effect, image));
      }

      /// Applies the effect to the image now, on the CPU.
      pub fn apply_on_cpu(&self, image: ::std::sync::Arc<$crate::Image>) {
        let effect = self.0.read().unwrap().clone();
        image.edit(|image| ::abra::options::prelude::Effect::apply_on_cpu(&effect, image));
      }

      /// Applies the effect to the image now, on the GPU.
      /// Panics if the effect has no GPU version, no GPU is there, or it fails.
      pub fn apply_on_gpu(&self, image: ::std::sync::Arc<$crate::Image>) {
        let effect = self.0.read().unwrap().clone();
        image.edit(|image| {
          ::abra::options::prelude::Effect::apply_on_gpu(&effect, image).unwrap_or_else(|message| panic!("{message}"))
        });
      }
    }
  };
  ($($handle:ident),+ $(,)?) => {$(
    #[uniffi::export]
    impl $handle {
      /// A copy of the effect limited to a mask: black is no effect, white is full effect, gray is partial. The mask must be
      /// the size of the image. It keeps the area and hardware the effect already had. Nothing removes the mask.
      pub fn with_mask(&self, mask: Option<::std::sync::Arc<$crate::Mask>>) -> ::std::sync::Arc<Self> {
        let effect = self.0.clone();
        ::std::sync::Arc::new(Self(::abra::options::prelude::Effect::with_mask(effect, mask.map(|mask| ::abra::abra_core::GrayPlane::from(mask.core())))))
      }

      /// A copy of the effect limited to an area, which may be feathered. It keeps the mask and hardware the effect already
      /// had. Nothing removes the area.
      pub fn with_area(&self, area: Option<::std::sync::Arc<$crate::Area>>) -> ::std::sync::Arc<Self> {
        let effect = self.0.clone();
        ::std::sync::Arc::new(Self(::abra::options::prelude::Effect::with_area(effect, area.map(|area| area.core()))))
      }

      /// A copy of the effect limited to several areas.
      pub fn with_areas(&self, areas: Vec<::std::sync::Arc<$crate::Area>>) -> ::std::sync::Arc<Self> {
        let effect = self.0.clone();
        let areas: Vec<_> = areas.iter().map(|area| area.core()).collect();
        let options = ::abra::options::prelude::Effect::options(&effect).clone().unwrap_or_default().with_areas(areas);
        ::std::sync::Arc::new(Self(::abra::options::prelude::Effect::with_options(effect, options)))
      }

      /// Applies the effect to the image now, on whichever hardware suits.
      pub fn apply(&self, image: ::std::sync::Arc<$crate::Image>) {
        image.edit(|image| ::abra::options::prelude::Effect::apply(&self.0, image));
      }

      /// Applies the effect to the image now, on the CPU.
      pub fn apply_on_cpu(&self, image: ::std::sync::Arc<$crate::Image>) {
        image.edit(|image| ::abra::options::prelude::Effect::apply_on_cpu(&self.0, image));
      }

      /// Applies the effect to the image now, on the GPU.
      /// Panics if the effect has no GPU version, no GPU is there, or it fails.
      pub fn apply_on_gpu(&self, image: ::std::sync::Arc<$crate::Image>) {
        image.edit(|image| {
          ::abra::options::prelude::Effect::apply_on_gpu(&self.0, image).unwrap_or_else(|message| panic!("{message}"))
        });
      }
    }
  )+};
}

pub(crate) use effect_methods;
