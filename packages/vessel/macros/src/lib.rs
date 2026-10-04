//! Derive macros for Vessel.
//!
//! `#[derive(Component)]` makes an app's own type a component that a host view can show, with nothing else to write.
//! It is re-exported by the `vessel` crate, so an app uses it as `vessel::Component`.
#![deny(missing_docs)]

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, Index, parse_macro_input};

/// The path to the Vessel crate that the code being compiled depends on: the `vessel` umbrella in an app, or `vessel-api`
/// in a crate such as `vessel-ui` that is part of the toolkit.
fn vessel_path() -> Result<TokenStream2, String> {
  for name in ["vessel", "vessel-api"] {
    match crate_name(name) {
      Ok(FoundCrate::Itself) => return Ok(quote!(crate)),
      Ok(FoundCrate::Name(found)) => {
        let ident = Ident::new(&found.replace('-', "_"), Span::call_site());
        return Ok(quote!(::#ident));
      }
      Err(_) => {}
    }
  }
  Err("add the `vessel` crate to this crate's dependencies".to_string())
}

/// Makes the type a component a host view can show.
///
/// The type holds a Vessel component in one of its fields: the one marked `#[component]`, or the first field. The derive
/// adds, for that component:
///
/// - `mount(view)` and `unmount(view)`, exported to JavaScript (with `uniffi`) for a `VesselView` to call, so the app
///   never writes mounting code or picks a view number;
/// - a `Drop` that takes it off every view it is on when it goes away;
/// - a `Deref` to the `Component`, so the type has the component's events, layout and settings itself.
///
/// The type also needs `uniffi::Object`, which is what exports its methods to the host.
///
/// A component that is only ever used inside others (a control such as a button, or a layout container) is never mounted
/// on a host view, so it marks itself `#[component(plain)]`, and the derive writes only the `Deref` and the check.
///
/// ```ignore
/// #[derive(uniffi::Object, Component)]
/// pub struct LiveView {
///   image: Image,
/// }
///
/// #[derive(Clone, Component)]
/// #[component(plain)]
/// pub struct Container {
///   component: Component,
/// }
/// ```
#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(p_input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(p_input as DeriveInput);
  let name = &input.ident;
  let vessel = match vessel_path() {
    Ok(path) => path,
    Err(message) => return syn::Error::new_spanned(name, message).to_compile_error().into(),
  };
  // `#[component(plain)]` on the struct: never mounted on a host view, so nothing is exported to it.
  let plain = input.attrs.iter().filter(|attribute| attribute.path().is_ident("component")).any(|attribute| {
    attribute.parse_nested_meta(|meta| if meta.path.is_ident("plain") { Ok(()) } else { Err(meta.error("expected `plain`")) }).is_ok()
  });
  if !input.generics.params.is_empty() {
    return syn::Error::new_spanned(&input.generics, "a component exported to a host view cannot be generic")
      .to_compile_error()
      .into();
  }
  let Data::Struct(data) = &input.data else {
    return syn::Error::new_spanned(name, "Component can only be derived for a struct").to_compile_error().into();
  };

  // The field that holds the component: marked `#[component]`, or else the first.
  let fields: Vec<_> = match &data.fields {
    Fields::Named(fields) => fields.named.iter().collect(),
    Fields::Unnamed(fields) => fields.unnamed.iter().collect(),
    Fields::Unit => Vec::new(),
  };
  let Some(position) = fields.iter().position(|field| field.attrs.iter().any(|attribute| attribute.path().is_ident("component"))).or(if fields.is_empty() { None } else { Some(0) }) else {
    return syn::Error::new_spanned(name, "the struct needs a field that holds the component").to_compile_error().into();
  };
  let field = match &fields[position].ident {
    Some(ident) => quote!(#ident),
    None => {
      let index = Index::from(position);
      quote!(#index)
    }
  };

  let field_type = &fields[position].ty;
  let hosted = if plain {
    quote!()
  } else {
    quote! {
      #[::uniffi::export]
      impl #name {
        /// Shows it on the native view `view`. A `VesselView` calls this.
        pub fn mount(&self, view: i32) {
          #vessel::mount(view, &self.#field);
        }

        /// Takes it off the native view `view`. A `VesselView` calls this when it goes away.
        pub fn unmount(&self, view: i32) {
          #vessel::unmount(view, &self.#field);
        }
      }

      impl ::std::ops::Drop for #name {
        fn drop(&mut self) {
          #vessel::unmount_all(&self.#field);
        }
      }
    }
  };
  quote! {
    // The field must be something the engine can render; this says so plainly if it is not.
    const _: () = {
      fn assert_renderable<T: #vessel::Renderable>() {}
      #[allow(dead_code)]
      fn check() {
        assert_renderable::<#field_type>();
      }
    };

    impl ::std::ops::Deref for #name {
      type Target = #vessel::Component;

      fn deref(&self) -> &#vessel::Component {
        #vessel::Renderable::component(&self.#field)
      }
    }

    #hosted
  }
  .into()
}
