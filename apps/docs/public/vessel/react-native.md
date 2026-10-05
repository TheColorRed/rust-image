---
title: Hosting in React Native
order: 2
outline: deep
---

# Hosting in React Native

On Android and iOS, `@vessel/react-native` provides `VesselView`, a React Native view that gives Vessel a native drawing surface. The Rust component remains responsible for its content; the host connects it to the view and forwards input.

## Configure the Rust library

Add the umbrella crate with the surface features for the platforms the app targets:

```toml
[dependencies]
vessel = { workspace = true, features = ["android-surface", "ios-surface"] }
```

Expose a component as a UniFFI object and derive `Component` for its renderable field:

```rust
use std::sync::Arc;
use vessel::prelude::*;

#[derive(uniffi::Object, Component)]
pub struct Preview {
  image: Image,
}

#[uniffi::export]
impl Preview {
  #[uniffi::constructor]
  pub fn new() -> Arc<Self> {
    Arc::new(Self { image: Image::new() })
  }
}
```

Add the native library name to the app's Android manifest, inside `<application>`:

```xml
<meta-data android:name="dev.vessel.library" android:value="your_rust_library" />
```

Use the library name produced by your app's Rust build. This metadata belongs to the app manifest: generated library manifests may be overwritten.

### iOS configuration

The iOS host uses a `CAMetalLayer`. Pixel-based components upload their RGBA frames through Metal; GPU-backed images can present directly from their existing GPU context without reading frames back through the CPU or JavaScript. Both paths preserve transparency for rounded corners and overlays.

Add the same library name to the app's `Info.plist`:

```xml
<key>dev.vessel.library</key>
<string>your_rust_library</string>
```

The binding loads `Frameworks/your_rust_library.framework/your_rust_library` from the app bundle. Build the Rust library with `ios-surface` enabled and package it as a dynamic XCFramework, using the same build pipeline as the UniFFI bindings. The native view and UniFFI must load the same library so they share Vessel's surface registry. Missing metadata, framework binaries, or bridge exports produce an explicit configuration error.

The repository's mobile app already sets this key to `alakazam_mobile` in its XcodeGen project. On macOS with Xcode installed, build its native bindings from the repository root:

```sh
npm exec --workspace packages/react-native/alakazam -- ubrn build jsi2 ios --release --and-generate --config ubrn.config.yaml
```

Abra's vendored JPEG library also needs the target-scoped CMake toolchain variables used by the repository's iOS CI job. When building locally, set `CMAKE_TOOLCHAIN_FILE_aarch64_apple_ios`, `CMAKE_TOOLCHAIN_FILE_aarch64_apple_ios_sim`, and `CMAKE_TOOLCHAIN_FILE_x86_64_apple_ios` to the absolute path of `.github/cmake/ios.cmake` before that command.

Generate the app project and install CocoaPods dependencies:

```sh
cd apps/mobile/ios
xcodegen generate --spec project.yml
pod install
```

The `VesselReactNative` pod is autolinked from `@vessel/react-native`. It compiles the view manager and links UIKit, QuartzCore, and Metal; the host library is not hard-coded in the pod. Open the generated workspace in Xcode to build and run the app.

Native iOS compilation, simulator rendering, and device testing must be performed on macOS. Verify image previews, rounded thumbnail transparency, slider dragging, rotation, source replacement, and background/foreground transitions before shipping.

## Show a component

Add the repository's `@vessel/react-native` package to the React Native app and pass the generated component object as `source`:

```tsx
import { VesselView } from '@vessel/react-native';

function PreviewPanel({ preview }: { preview: Preview | null }) {
  if (!VesselView || !preview) return null;
  return <VesselView source={preview} style={{ width: 320, height: 240 }} />;
}
```

`VesselView` assigns a view id and calls the source's generated `mount` and `unmount` methods as it appears, changes source, or leaves the React tree. The app does not choose surface ids or open a Vessel `Session`. Set its React Native style to determine the view's size.

`VesselView` is available on Android and iOS and is `null` on unsupported platforms. Both hosts forward touch input to the mounted component, including the pointer events used by controls such as `Slider`. The iOS host converts view points to physical pixels using the display scale, matching the component's drawable size.

The wrapper also supplies the host's font before mounting and applies display density and accessibility font scaling. On iOS the view manager obtains the system font's file URL through CoreText when it is readable. If the operating system does not expose that file, the host logs the fallback; pass a readable local TrueType/OpenType path with `fontPath` to supply an outline face. `fontSize` overrides the logical size, and `allowFontScaling={false}` disables accessibility scaling. Explicit font-loading failures are reported rather than silently ignored.

The iOS surface is retired when the view leaves its window or the app becomes inactive and registered again when it returns. Vessel retains the Metal layer until rendering has stopped, so removing or replacing a view cannot free a layer while a frame is being presented.

## Keep messages and rendering in Rust

Use a component's typed message stream for values sent from JavaScript. For example, an image preview can accept a `Message` enum, apply the change to its source, and let the engine redraw it. A slider component can also subscribe directly to its changes and send preview messages without routing every drag update through JavaScript.

This pattern keeps platform code at the host boundary:

1. JavaScript creates the UniFFI component and gives it to `VesselView`.
2. Rust connects the component to its image or other source.
3. The host provides the surface and forwards input.
4. The engine schedules drawing; frames do not need to pass through JavaScript.

See `packages/react-native/alakazam/src/components/image_preview.rs` for a component that tracks an image source and consumes typed preview messages.
