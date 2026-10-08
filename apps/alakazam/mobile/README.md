# apps/alakazam/mobile

Alakazam's React Native application, backed by the Rust library.

## Android development

Run these commands from the repository root:

```sh
npm ci
npm run start:android
```

In another terminal, build and deploy to a connected Android device:

```sh
npm run deploy:android
```

For a standalone release with the bundled ONNX runtime and JavaScript:

```sh
npm run deploy:android -- --release --targets arm64-v8a
```

Rebuild the native app after AI dependencies change; an older APK will not gain
the runtime from a JavaScript reload. Person-detection failures display the native
error explanation rather than only the `AbraError.Ai` variant.

## Selecting people for skin effects

Open **Beauty > Skin Smooth** or **Beauty > Skin Tan** to show detected people
and the person-selection controls alongside the effect's slider or color picker.
Tap a person in the preview to target them, or choose **All people** to apply
skin effects to everyone. Leaving the skin control hides the selection UI and
outlines; other tabs keep their normal controls.
Touching the skin slider or color picker hides the outlines without clearing the
selected person, so the effect is unobstructed. Tap the photo again to show them.
Each person keeps their own Smooth and Tan values. Switching people changes the
target and slider value, without moving or removing edits already made to others.
Replaying edits uses explicit person masks without changing the live selection,
and the completed image replaces the preview after the entire stack is rendered.
Live preview frames are rendered off-screen on Vessel's engine thread. Pending
changes are coalesced to the newest pending state while completed frames keep
displaying during continuous drags. GPU output is copied to a stable off-screen texture before publication
(without CPU readback), so the displayed frame cannot be overwritten by the next
render. CPU-only effects use the same off-screen scheduling.
On release, the live effect stays visible until Vessel confirms that the
committed photo frame reached the surface; there is no fixed-delay handoff.

Run the skin-adjustment regression tests from the repository root:

```sh
node --test apps/alakazam/mobile/tests/skin-adjustments.test.cjs apps/alakazam/mobile/tests/preview-handoff.test.cjs
```

## Unsigned iOS test builds

Run the **Build downloadable apps** GitHub Actions workflow with iOS enabled.
The iOS job builds the Rust XCFramework, generates the native app project with
XcodeGen, installs CocoaPods, and archives the React Native app with signing
disabled. It uploads `Alakazam-ios-unsigned.ipa` directly, without an extra ZIP
wrapper. No Apple signing certificate or provisioning profile is needed to build
this artifact.

An IPA is itself a ZIP containing `Payload/Alakazam.app`. This unsigned file must
be signed by a sideloading tool before installation on an iPhone. It is not a
TestFlight or App Store build.

The iOS app host is initial testing infrastructure. Vessel's native iOS surface
is not implemented yet, so native image previews and the skin-tan picker are
unavailable on iOS. Producing an IPA does not imply feature parity with Android.

The source of the Xcode project is `ios/project.yml`; generated Xcode projects,
workspaces, Pods, and build outputs remain ignored. Compiling and running the
iOS app requires macOS and Xcode.
