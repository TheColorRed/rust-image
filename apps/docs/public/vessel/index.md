---
title: Vessel
order: 0
outline: deep
---

# Vessel

Vessel is a toolkit for building reusable components and applications that draw themselves. An app describes its components, state, and layout; the engine handles scheduling and rendering. The same component API can be hosted in a standalone window or inside a view owned by another application.

Vessel is separate from Abra. The toolkit does not depend on Abra, so an app can connect its own image or media source through Vessel's interfaces without adding image-processing dependencies to the engine.

## How an app fits together

- A **component** draws content and can contain child components.
- **Properties** represent current state; changing one requests the redraws that depend on it.
- **Streams** carry typed events and state changes. Components do not need a separate event bus.
- A **host** supplies a drawing surface and translates platform input. The engine decides when to draw.

Most Rust apps depend on the umbrella `vessel` crate, which re-exports the engine, API, controls, and derive macro. Begin with:

```rust
use vessel::prelude::*;
```

Inside this repository, add Vessel as a workspace dependency. Enable only the feature for the host the app needs:

```toml
[dependencies]
vessel = { workspace = true, features = ["android-surface"] }
```

Features are off by default. The currently available hosts are:

| Feature | Host | Availability |
| --- | --- | --- |
| `android-surface` | Draw inside an Android view owned by another app, such as React Native. | Used by the mobile app. |
| `ios-surface` | Draw inside an iOS view owned by another app through a Metal layer. | Implemented; native build and device verification require macOS/Xcode. |
| `desktop-window` | Open a standalone desktop window. | Available on Windows. |

Standalone native Android and iOS window hosts are not available yet.

## Guides

| Guide | Covers |
| --- | --- |
| [Building components](./components) | Layout, ready-made controls, typed events, and image-backed components. |
| [Hosting in React Native](./react-native) | Exposing a Rust component to JavaScript and showing it with `VesselView` on Android or iOS. |

## Try an example

The `vessel-panels` example shows a flex layout, labels, buttons, a slider, and a component that redraws from the controls' state:

```sh
cargo run -p vessel-panels-test --bin vessel-panels
```

Press Escape to close the window. The example needs the desktop window feature and does not use Abra.
