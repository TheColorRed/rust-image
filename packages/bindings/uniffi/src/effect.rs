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

      /// The effect as a live effect, for `Layer::set_live_effects`: a copy of it as it is now.
      pub fn live_effect(&self) -> ::std::sync::Arc<$crate::LiveEffect> {
        let effect = self.0.read().unwrap().clone();
        ::std::sync::Arc::new($crate::LiveEffect::new(::std::sync::Arc::new(effect)))
      }

      /// Applies the effect to the image now, on whichever hardware suits.
      pub fn apply(&self, image: ::std::sync::Arc<$crate::Image>) {
        let effect = self.0.read().unwrap().clone();
        image.edit(|image| ::abra::options::prelude::Effect::apply(&effect, image));
      }

      /// A copy of the image with the effect applied, and the image itself left as it was.
      pub fn applied(&self, image: ::std::sync::Arc<$crate::Image>) -> ::std::sync::Arc<$crate::Image> {
        let effect = self.0.read().unwrap().clone();
        ::std::sync::Arc::new($crate::Image::from_core(::abra::options::prelude::Effect::applied(&effect, &image.core())))
      }

      /// Applies the effect to the image later, as the library's `apply(deferred(&mut image))`: the image remembers the effect
      /// and the GPU does it when the picture is drawn, in one run with the effects after it. The effect is applied now when
      /// it cannot wait (no shader, an area or a mask, set to the CPU, or no GPU).
      pub fn apply_deferred(&self, image: ::std::sync::Arc<$crate::Image>) {
        let effect = self.0.read().unwrap().clone();
        image.edit(|image| ::abra::options::prelude::Effect::apply(&effect, ::abra::options::prelude::deferred(image)));
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

      /// The effect as a live effect, for `Layer::set_live_effects`: a copy of it as it is now.
      pub fn live_effect(&self) -> ::std::sync::Arc<$crate::LiveEffect> {
        ::std::sync::Arc::new($crate::LiveEffect::new(::std::sync::Arc::new(self.0.clone())))
      }

      /// Applies the effect to the image now, on whichever hardware suits.
      pub fn apply(&self, image: ::std::sync::Arc<$crate::Image>) {
        image.edit(|image| ::abra::options::prelude::Effect::apply(&self.0, image));
      }

      /// A copy of the image with the effect applied, and the image itself left as it was.
      pub fn applied(&self, image: ::std::sync::Arc<$crate::Image>) -> ::std::sync::Arc<$crate::Image> {
        ::std::sync::Arc::new($crate::Image::from_core(::abra::options::prelude::Effect::applied(&self.0, &image.core())))
      }

      /// Applies the effect to the image later, as the library's `apply(deferred(&mut image))`: the image remembers the effect
      /// and the GPU does it when the picture is drawn, in one run with the effects after it. The effect is applied now when
      /// it cannot wait (no shader, an area or a mask, set to the CPU, or no GPU).
      pub fn apply_deferred(&self, image: ::std::sync::Arc<$crate::Image>) {
        image.edit(|image| ::abra::options::prelude::Effect::apply(&self.0, ::abra::options::prelude::deferred(image)));
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

/// An effect a layer holds as live settings, made by an effect's `live_effect`. See `Layer::set_live_effects`.
#[derive(uniffi::Object)]
pub struct LiveEffect(::std::sync::Arc<dyn ::abra::abra_core::image::gpu::LiveEffect>);

impl LiveEffect {
  pub(crate) fn new(effect: ::std::sync::Arc<dyn ::abra::abra_core::image::gpu::LiveEffect>) -> Self {
    Self(effect)
  }

  pub(crate) fn core(&self) -> ::std::sync::Arc<dyn ::abra::abra_core::image::gpu::LiveEffect> {
    self.0.clone()
  }
}
