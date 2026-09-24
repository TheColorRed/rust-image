# Naming Conventions

- Add the name of the object a function will operate on within the function name when not operating on itself.
  - ```rust
      // Correct
      pub fn set_layer_opacity(&mut self, p_layer: &Layer, p_opacity: f32) { ... }
      pub fn get_layer(&self, p_index: usize) -> Option<Layer> { ... }

      // Incorrect
      pub fn set_opacity(&mut self, p_layer: &Layer, p_opacity: f32) { ... }
      pub fn layer(&self, p_index: usize) -> Option<Layer> { ... }
    ```

## Public API vs Internal

- Use `Inner` suffix for internal items to differentiate from public API items. However, inner items can be public, but should not be exported to the public API.
  - ```rust
      // Correct
      pub struct Canvas { ... }      // Public User Facing API
      pub struct CanvasInner { ... } // Internal to the library

      // Incorrect
      pub struct Canvas { ... }        // Public User Facing API
      pub struct CanvasPrivate { ... } // Internal to the library
    ```

- Use clean simple names for public API items. Avoid abbreviations unless they are widely recognized.
  - ```rust
      // Correct
      pub fn resize_canvas(&mut self, p_width: u32, p_height: u32) { ... }

      // Incorrect
      pub fn res_canvas(&mut self, w: u32, h: u32) { ... }
    ```

## Getters

- Use the `get_` prefix for methods that get an item from a collection.
- Don't use `get_` prefix for methods that return a value from itself.
  - ```rust
      // Correct
      pub fn get_layer(&self, p_index: usize) -> Option<Layer> { ... }
      pub fn opacity(&self) -> f32 { ... }

      // Incorrect
      pub fn layer(&self, p_index: usize) -> Option<Layer> { ... }
      pub fn get_opacity(&self) -> f32 { ... }
    ```

## Setters

- Always use the `set_` prefix for setter methods.

## Function names

- `as_*` for functions that return a reference or view of the object in a different form without consuming or cloning it.
- `to_*` for functions that return a new owned instance of the object in a different form, typically involving cloning or conversion.
- `with_*` for builder-style methods that return a modified instance of the object, often used in method chaining.
- `is_*` for functions that return a boolean indicating a state or property of the object.
- `has_*` for functions that return a boolean indicating the presence of a feature or component within the object.
- `from_*` for constructor functions that create an instance of an object from another type or representation.
- `into_*` for functions that consume the object and convert it into another type or representation.

## Function parameters

- Prefix every function parameter with `p_` (**p** for **Parameter**). This makes it clear at a glance which names are inputs to the function and which are local variables. It also lets locals reuse the plain name without conflicting with the parameter.
- The rule applies to free functions, methods, associated functions, and trait methods, public or internal.
- `self`, `&self`, and `&mut self` are never prefixed.
- Intentionally unused parameters keep Rust's `_` prefix (e.g. `_options`, or just `_`) instead of `p_`. The compiler only silences the unused-variable warning for names that start with `_`, so a `p_` name would bring the warning back.
- Use the prefix only on the parameter. Don't carry it over to locals or struct fields.
  - ```rust
      // Correct
      fn resize_image(&mut self, p_width: u32, p_height: u32, p_algorithm: Option<ResizeAlgorithm>) {
          // Locals can use the plain names without clashing with the parameters
          let (width, height) = self.dimensions();
          let algorithm = p_algorithm.unwrap_or_default();
      }

      // Incorrect
      fn resize_image(&mut self, width: u32, height: u32, algorithm: Option<ResizeAlgorithm>) {
          // Locals need awkward names, and parameters look the same as locals
          let (current_width, current_height) = self.dimensions();
      }
    ```
