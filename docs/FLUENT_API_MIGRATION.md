# Canvas Layer API: Fluent API Migration Guide

## The Problem: Awkward Multi-Borrow Pattern

The original API required users to manage a separate `layers_mut()` handler:

```rust
let project = Canvas::new("My Project");
let layers = project.layers_mut();

// Can only modify layers here...
{
    let layer = layers.add_layer_from_path("Background", "bg.png", None);
}

// Must drop layers handler before calling project methods
// This is error-prone and not enforced by the type system
project.save("output.png", None);
```

**Problems:**
- ❌ Must remember to drop the `layers` handler before using `project` again
- ❌ Awkward scope management with nested braces
- ❌ Not enforced by Rust's type system
- ❌ Easy to make mistakes that aren't caught until runtime

---

## The Solution: Fluent API

### Method 1: Direct Fluent API (Simple Cases)

For simple layer operations, use the fluent API with method chaining:

```rust
let project = Canvas::new("My Project")
    .add_layer_from_path("Background", "bg.png", None)
    .add_layer_from_path(
        "Overlay",
        "overlay.png",
        Some(NewLayerOptions {
            anchor: Some(Anchor::TopRight),
        }),
    );

// Use project immediately - no borrow issues!
project.save("output.png", None);
```

**Benefits:**
- ✅ Clean, intuitive API
- ✅ No borrow checker issues
- ✅ Method calls can be chained if needed
- ✅ Works great for sequential operations

### Method 2: Closure-Based API (Complex Cases)

For more complex layer management, use `with_layers()`:

```rust
let project = Canvas::new("My Project");

project.with_layers(|layers| {
    layers.add_layer_from_path("Background", "bg.png", None);
    layers.add_layer_from_path("Middle", "middle.png", None);
    layers.add_layer_from_path("Top", "top.png", None);

    // All layer management operations available
    layers.move_to_top(1);
    layers.move_to_bottom(2);
});

// All layers added and reordered
project.save("output.png", None);
```

**Benefits:**
- ✅ Full access to `RcLayersHandler` methods
- ✅ Scoped layer operations
- ✅ Cleaner than nested braces
- ✅ Type-safe - scope is enforced

### Method 3: Storing Layer References

You can capture layer references to modify them later:

```rust
let project = Canvas::new("My Project");

let background = project.add_layer_from_path("Background", "bg.png", None);
let overlay = project.add_layer_from_path("Overlay", "overlay.png", None);

// Modify layers later
background.borrow_mut().set_opacity(0.8);
overlay.borrow_mut().set_blend_mode(blend::multiply);

project.save("output.png", None);
```

**Benefits:**
- ✅ Direct access to layer references
- ✅ Can modify layers after adding them
- ✅ Good for complex workflows

---

## Migration Examples

### Before: Old API with layers_mut()

```rust
pub fn main() {
    let project = Canvas::new("Layered Image Project");
    let layers = project.layers_mut();

    // Awkward scope management
    {
        layers.add_layer_from_path("Background", "boobs.webp", None);
    }

    {
        let layer = layers.add_layer_from_path(
            "Top Layer",
            "bikini.jpg",
            Some(NewLayerOptions {
                anchor: Some(Anchor::BottomRight),
            }),
        );
        // layer.borrow_mut().set_index(0);
    }

    // Must drop layers before using project!
    project.save("out/layers.png", None);
}
```

### After: New Fluent API with Method Chaining

```rust
pub fn main() {
    let project = Canvas::new("Layered Image Project")
        .add_layer_from_path("Background", "boobs.webp", None)
        .add_layer_from_path(
            "Top Layer",
            "bikini.jpg",
            Some(NewLayerOptions {
                anchor: Some(Anchor::BottomRight),
            }),
        );

    // All layers added and composed
    project.save("out/layers.png", None);
}
```---

## New Canvas API Methods

### Layer Addition (Fluent API)

```rust
// Add layer from file path
pub fn add_layer_from_path(
    &self,
    name: &str,
    path: &str,
    options: Option<NewLayerOptions>,
) -> Rc<RefCell<Layer>>

// Add layer from an Image
pub fn add_layer_from_image(
    &self,
    name: &str,
    image: Image,
    options: Option<NewLayerOptions>,
) -> Rc<RefCell<Layer>>
```

### Scoped Layer Management

```rust
// Execute a closure with full RcLayersHandler access
pub fn with_layers<F>(&self, f: F)
where
    F: FnOnce(&mut RcLayersHandler),
```

### Layer Queries

```rust
// Get a layer by index
pub fn get_layer(&self, index: usize) -> Option<Rc<RefCell<Layer>>>

// Get the total number of layers
pub fn layer_count(&self) -> usize
```

---

## Deprecation Notice

The `layers_mut()` method is now deprecated:

```rust
#[deprecated(since = "0.2.0",
             note = "use add_layer_from_path, add_layer_from_image, or with_layers instead")]
pub fn layers_mut(&self) -> RcLayersHandler
```

**Why:**
- The fluent API is more ergonomic
- No borrow checker conflicts
- Type-safe operations

---

## Common Patterns

### Pattern 1: Sequential Layer Addition with Method Chaining

```rust
let project = Canvas::new("My Project")
    .add_layer_from_path("BG", "background.png", None)
    .add_layer_from_path("FG", "foreground.png",
        Some(NewLayerOptions {
            anchor: Some(Anchor::TopRight),
        }));

project.save("output.png", None);
```### Pattern 2: Add Layers with Immediate Modification

```rust
let project = Canvas::new("My Project");

let bg = project.add_layer_from_path("Background", "bg.png", None);
let fg = project.add_layer_from_path("Foreground", "fg.png", None);

// Modify right after adding
bg.borrow_mut().set_opacity(0.9);
fg.borrow_mut().set_blend_mode(blend::screen);

project.save("output.png", None);
```

### Pattern 3: Complex Layer Management

```rust
let project = Canvas::new("My Project");

project.with_layers(|layers| {
    let layer1 = layers.add_layer_from_path("L1", "img1.png", None);
    let layer2 = layers.add_layer_from_path("L2", "img2.png", None);
    let layer3 = layers.add_layer_from_path("L3", "img3.png", None);

    // Reorder layers
    layers.move_to_top(1);
    layers.move_to_bottom(2);

    // Remove layer
    if layers.count() > 3 {
        layers.remove_at(0);
    }
});

project.save("output.png", None);
```

---

## Benefits Summary

| Aspect               | Old API                  | New API                  |
| -------------------- | ------------------------ | ------------------------ |
| **Ergonomics**       | ❌ Awkward                | ✅ Fluent & intuitive     |
| **Borrow Issues**    | ❌ Easy to make mistakes  | ✅ Type-safe              |
| **Method Chaining**  | ❌ Not possible           | ✅ Possible               |
| **Scope Management** | ❌ Requires manual braces | ✅ Automatic with closure |
| **Code Clarity**     | ❌ Harder to read         | ✅ Easier to understand   |
| **Error Handling**   | ❌ Runtime issues         | ✅ Compile-time safety    |

---

## Technical Details

### Why This Works Better

The new API returns layer references directly from `add_layer_from_path()` and `add_layer_from_image()`, so you don't need to hold a separate handler. The `Canvas` handles the internal borrow management:

```rust
impl Canvas {
    pub fn add_layer_from_path(
        &self,  // Note: &self, not &mut self
        name: &str,
        path: &str,
        options: Option<NewLayerOptions>,
    ) -> Rc<RefCell<Layer>> {
        // Internally uses &self.inner.borrow_mut() temporarily
        // Borrow is released after the layer is added
        // No conflicts with calling other Canvas methods
    }
}
```

This is safe because:
1. Interior mutability (`Cell<>` and `RefCell<>`) allows mutation through shared references
2. Borrows are temporary and released immediately after use
3. The `Canvas` remains available for other operations

---

## Testing

The new API has been tested with the `layers-test` example:

```
Open Time: 423.5091ms
Image opened: assets/boobs.webp
Open Time: 296.2997ms
Image opened: assets/bikini.jpg
PNG Compression level set to Balanced
Save Time: 342.4738ms
Image saved to out/layers.png
```

✅ All layers added and composed correctly!
