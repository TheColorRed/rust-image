# Public API

Abra is a library, so it has a public facing API that allows other programs to interact with its functionality.

## Usage

The api should be as simple to use as possible, users should not have to write complex code to perform any operation.

## API Design Principles

- Prefer owned values over requiring callers to manage `Arc`, `Mutex`, or other shared-state wrappers.
- Accept flexible inputs with `Into`, `impl Trait`, and `Option` where this keeps common calls concise.
- Keep common operations one or two calls, with sensible defaults for optional behavior.
- Hide internal locking, borrowing, reference conversions, and synchronization details behind public methods.
- Provide builders for genuinely complex configuration, but do not require them for simple operations.
- Return useful wrapper types that expose the next common operation directly, such as mutating a layer after adding it.

## Ownership and Mutation

- Public methods should take shared references when the operation does not require ownership of the wrapper.
- Manage internal `Arc` and `Mutex` values inside the library instead of exposing them to callers.
- Prefer in-place mutation for filters, adjustments, and drawing operations when the caller owns the image.
- Use short-lived closure-based mutation, such as `with_image_mut`, when mutating data managed by a layer or canvas.
- Clone an image only when the caller needs an independent image or must preserve the original; do not clone merely to perform an edit.
- Return newly created layers, images, or other useful handles so callers can continue working without reaching into internal state.

## Example 1

As seen here, the user does not have to manually convert a `Color` into the required type for `some_function`. Instead the library does it automatically.

```rust
// Too complex
some_function(Color::new(255, 0, 0).into());

// Simpler alternative
some_function(Color::red());
```

## Example 2

The user should not have to worry about ownership and borrowing when using the library's functions. The library should handle these details internally to provide a smooth and simple API.

```rust
// Too complex for a canvas
let canvas = Arc::new(Mutex::new(Canvas::new_blank("My Project", 800, 600)));
let layer = {
	let canvas_ref = canvas.lock().unwrap();
	canvas_ref.add_layer_from_path("Background", "assets/background.png", None)
};
layer.with_image_mut(|image| {
	// Mutate the image here.
});
canvas.lock().unwrap().save("out/background.png", None);

// Simpler alternative
let canvas = Canvas::new_blank("My Project", 800, 600);
let layer = canvas.add_layer_from_path("Background", "assets/background.png", None);
layer.with_image_mut(|image| {
	// Mutate the image here; Abra manages the borrow internally.
});
canvas.save("out/background.png", None);

```

## Example 3

The public API should avoid making callers clone an image and replace it just to
perform an in-place edit.

```rust
// Too complex
let mut image = layer.clone_image();
// Mutate `image` here.
layer.set_image(Arc::new(image));

// Simpler alternative
layer.with_image_mut(|image| {
	// Mutate the existing image in place.
});
```

## Example 4

The public API should accept ordinary owned values when it can manage shared
ownership internally.

```rust
// Too complex
let image = Arc::new(Image::new_from_path("assets/background.png"));
let canvas = Canvas::new("My Project");
canvas.add_layer_from_image("Background", image, None);

// Simpler alternative
let canvas = Canvas::new("My Project");
canvas.add_layer_from_path("Background", "assets/background.png", None);
```

## Example 5

The public API should provide high-level constructors for common workflows
instead of requiring callers to assemble the same objects manually.

```rust
// Too complex
let canvas = Canvas::new("My Project");
let image = Image::new_from_path("assets/background.png");
canvas.add_layer_from_image("Background", image, None);

// Simpler alternative
let canvas = Canvas::new_from_path("My Project", "assets/background.png", None);
```

## Example 6

Image operations should accept an image directly instead of requiring callers
to construct an intermediate reference type.

```rust
// Too complex
let mut image = Image::new_from_path("assets/background.png");
let image_ref: ImageRef = (&mut image).into();
blur(image_ref, None);

// Simpler alternative
let mut image = Image::new_from_path("assets/background.png");
blur(&mut image, None);
```

## Example 7

Drawing APIs should provide convenient defaults so callers only configure the
properties that matter for their operation.

```rust
// Too complex
let brush = Brush::new()
	.with_size(32)
	.with_hardness(1.0)
	.with_opacity(1.0)
	.with_color(&Color::blue());

// Simpler alternative
let brush = Brush::new().with_size(32).with_color(&Color::blue());
```
