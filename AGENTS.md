Always look for and read one or more skills that are related to the task you are performing.

## Building

- NASM is in the project at `tools/nasm` (`tools/nasm/nasm.exe`). The `rav1d` crate needs it to build, and it is not on the system PATH. Put it on the PATH for the command that builds, for example in Git Bash: `export PATH="$PATH:$PWD/tools/nasm"`.
- Never run `cargo clean` without `-p <crate>`: with no package it deletes the whole `target` folder (tens of GB, and the Android libraries) and everything has to compile again.

## Abra Is Not Only For Apps

- Abra is a standalone library for any situation: headless servers, CLIs, batch jobs, WebAssembly, and desktop or mobile apps. Library features must work from plain Rust with no app, UI, plugin host or bindings, and the library never depends on an app. See the abra-library skill.

## Legacy Code

- When a better way to achieve something exists, replace the old approach with the new one instead of keeping both.
- Prefer faster, more efficient code over legacy approaches, even if that means rewriting working code.
- Prefer approaches that give higher quality image results over legacy approaches.
- Rewrite dependencies and code that rely on a legacy approach to use the new, preferred method, for example by changing the signature and updating every caller rather than leaving a shim for the old one.

## Performance

- Running on a reduced copy of an image (a proxy, a thumbnail, a pre-resized photo) to make something fast is a last resort. First make the work on the full-quality image cheaper: keep decoded images between rebuilds, cache results that did not change, run on the GPU, and skip work that is not needed. A reduced copy gives up quality, so use one only when none of those is enough, and say so when you do.
- Fixing the speed on a full-size image also makes smaller images much faster, so tune and measure against the full-size case.

## Stability: Freezes, Deadlocks And Memory Growth (Watch For These In Every Task)

This is a standing sub-task of every other task. While you read or change code for any reason, keep an eye out for the things below. When you see one in code you are touching, fix it as part of the work; when it is somewhere else, tell the user where it is and what could happen. These came from a real incident: the mobile app froze after hours of use because of a lock-order deadlock in the `canvas` crate, and the same class of bug turned up a second time in the same crate.

### Deadlocks and freezes

- **Lock order.** When code holds one `Mutex` or `RwLock` and then takes another, every other path must take them in the same order. The canvas and its layers are the example: composing takes the canvas lock and then each layer's lock, so a layer method must never take the canvas lock while it holds its own. Write the order in a comment where the locks are defined.
- **Do not hold a lock while calling out.** Read what you need, release the guard, then call the other object. Do not hold a guard across a call into another lock, a callback, a subscriber, a channel send or receive, a GPU wait, or a JS or bindings call.
- **Prefer no lock over a second lock.** A flag one thread sets and another reads is an atomic (see `DirtyFlag` in the `canvas` crate), not a value behind a second mutex. If you move a "clear" and a "set" onto an atomic, clear before the work that reads, so a change made during the work is not lost.
- **Same-thread relocking.** `std` mutexes are not re-entrant: locking a mutex you already hold on the same thread hangs. Check for it when a method locks `self` and then calls something that can lock `self` again, or when an object can be added to itself or to its own parent.
- **Guards that live long.** A reference type that keeps a guard inside it (such as `ImageRef` from a `&Layer`) keeps the lock for as long as it lives. Do not keep one across a call into another lock.
- **Poisoned locks.** `lock().unwrap()` turns one panic into a panic on every later call. Where a panic while holding a lock is possible (asserts, indexing), prefer `unwrap_or_else(|poisoned| poisoned.into_inner())` if the data stays valid, as `OUTLINES` in `stroke.rs` does.
- **Background and engine threads.** Anything that runs on the Vessel engine thread, the GPU thread or a worker must never wait on a lock the UI or JS thread can hold while waiting on that thread.
- **Prove it with a test.** A fix for a deadlock gets a regression test with two threads and `mpsc::channel` plus `recv_timeout`, so a hang fails the test instead of hanging the run. Check the test fails against the old code before trusting it (see the `canvas` tests `changing_a_layer_while_the_canvas_composes_does_not_wait_forever` and `asking_a_layer_for_its_place_while_the_canvas_composes_does_not_wait_forever`).

### Memory growth

- **Anything kept per frame or per call must have a limit.** A cache, a list of past frames or images, a pool, a history or a channel queue needs a maximum size and a way to drop the oldest, like `OUTLINES_KEPT` and `MAX_IDLE_RENDERERS`. Flag a `static`, a `Vec` or a `HashMap` that is only ever pushed to.
- **No reference cycles.** An `Arc` that holds something that holds the same `Arc` back is never freed. Check cached values hung off an object (such as `Image::derived`) and closures that capture the thing that owns the closure.
- **Free native objects the code made.** In the plugins and the app, an object made through the bindings every frame (an effect, a color, an image) is released when the frame is done (`release(...)`); do not rely on the JS engine's garbage collector to free native memory, because it cannot see how big the native side is.
- **Per-frame work in JS.** Do not build big strings (`JSON.stringify` of a whole system), arrays or closures on every frame when they can be made once and reused.
- **Do not turn on tracing or leak the setup.** Settings or properties you change on the device to measure (for example `persist.heapprofd.enable`) are put back when the measurement is done.

### Measuring on the phone or tablet

- A long run is the only real test for these, so for anything that runs continuously (the particle maker, live previews, video, game loops) say how long it was watched. A short run proves little.
- Count `Pss` plus `SwapPss`, not `Pss` alone (a swapped-out app looks smaller than it is), and compare the same categories over time (`dumpsys meminfo`: Native Heap, Graphics, Unknown). A native heap that drops back in steps is garbage waiting to be freed; one that only climbs is a leak.
- Native allocations can be profiled with Perfetto `heapprofd` on the debug build and turned into function names with the NDK's `llvm-symbolizer` against `target/aarch64-linux-android/debug/libalakazam_mobile.so`. The profile only gives a final snapshot, so also sample `dumpsys meminfo` over time.
- A freeze is found with `dumpsys activity exit-info`, `dumpsys input`, a `kill -3` thread dump through `run-as` and `adb bugreport` (the trace is in `FS/data/anr/`), not by guessing. Two threads each waiting on a lock the other holds is a deadlock.
- Unresolved at the time of writing: the JavaScript engine heap in the mobile app grew about 20 MB an hour during a long run. That run only had the particle maker open, so it is **not known** where the growth is: it may be the particle maker, the plugin host, the preview, React Native or the app's own state, and it may happen in other screens too. Do not assume it is the particle maker. Watch for anything in the app's JS that is added to over time and never removed (an array, a `Map`, a subscription, a closure, a log buffer, an rxjs stream that is never unsubscribed), in whatever code you touch. A heap snapshot with Chrome DevTools, taken at the start and again hours later, would show which objects pile up, and a long run of a different screen would show whether the growth follows the particle maker or the app.
