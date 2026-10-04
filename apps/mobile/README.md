# apps/mobile

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
