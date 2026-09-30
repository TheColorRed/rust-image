# Public API Render Pipeline

An effect is applied the same way to an image or to a live image: `effect.apply(&mut target)`. What happens next
depends on the target.

## Applying to an image

1. Create the image: `let mut image = Image::from_*(...)`
2. Apply an effect: `brightness(...).apply(&mut image)`
3. The image is edited right away, in one of two ways:
   - **GPU:** used when the effect has a GPU version, the GPU is enabled in settings, and the GPU is ready.
   - **CPU:** used in every other case.
4. If the effect has an area or a mask, only that part of the image changes.

## Applying to a live image

A `LiveImage` is the way to show any edit on screen: sliders, buttons, pickers, and anything else that changes what
the image looks like.

1. Create it from RGBA pixels: `let mut live = LiveImage::new(width, height, pixels)?`
   - It picks its backend once, here. It uses the GPU if it is enabled and a GPU session can be created.
     Otherwise it uses the CPU.
2. Apply an effect: `brightness(...).apply(&mut live)`
   - The effect is added to the end of the live image's list of effects. It does not change the original pixels.
   - Applying the same kind twice adds two effects, just like applying it twice to an image.
   - Effects run in list order. Every apply re-renders the whole list from the original pixels.
3. Change an effect that is already in the list, such as while a slider is dragged:
   - Get an id with `live.new_id()`, then `brightness(...).apply(live.slot(id))`. The first apply with an id adds the
     effect, and every later one replaces it and keeps its place in the list.
   - `live.remove(id)` takes an effect out, `live.move_to(id, index)` changes when it runs, and `live.ids()` lists
     them in order. An id stays with its effect when others are removed or moved, which a list position would not.
4. It renders the list:
   - **GPU backend:** the render is started and `apply` returns straight away. If new values arrive before it
     finishes, only the newest are drawn.
     An effect with no GPU version runs on the CPU in between the GPU steps.
   - **CPU backend:** `apply` finishes the render before it returns, so call it from a worker thread.
5. Get the result in one of two ways:
   - **Poll:** call `live.poll()` on a timer. It returns the newest new frame, or nothing if there isn't one. It
     never waits.
   - **Present:** on Android, `present_to_android_window` draws frames straight onto the screen from the GPU, so no
     pixels are copied to the CPU. `poll()` returns nothing while presenting.
6. `live.clear()` removes all effects and shows the original image.

## Good to know

- Masks and areas work in live images too. An effect keeps the area and mask it was given, runs over the whole image,
  and its result is mixed with its input by a per-pixel weight (area edges, feathering and mask brightness), which is
  the same weight the image path uses. On the GPU that is one extra pass after the effect's own passes.
- Where areas overlap, the live image uses the strongest weight, while the image path blends the areas one after the
  other, so the two can differ slightly on the overlap.
- A render error is not returned by `apply`. It is returned by the next `poll()`.
- Adding a new effect: implement `Apply` (its `apply_to_image` does the work, with the area and mask handling) and
  derive `Clone`. Then make it usable everywhere, including live images:
  - CPU only: `options::cpu_processor!(MyEffect);`
  - With a shader: implement `CpuProcessor` (`process` calls `apply_to_image`, and `gpu()` returns `Some(self)`) and
    `GpuProcessor` (`passes` returns the shader and this frame's uniforms). See `Exposure` for an example.
  - To use it from the bindings, add it to `EffectSpec` in `abra/abra/src/live.rs`.
