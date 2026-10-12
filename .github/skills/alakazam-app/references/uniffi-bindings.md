# Bindings

## Library integration/bindings

The mobile app in `/apps/mobile` uses the Abra library for image manipulation through the `@alakazam/mobile` package in `/packages/react-native/alakazam`. The package's bindings are generated with `uniffi-bindgen-react-native` using its `jsi2` flavour. They include the `libalakazam_mobile.so` native library, generated TypeScript files, Android integration files, and a CocoaPods specification. `apps/mobile` depends directly on `@ubjs/react-native` so React Native autolinking installs the runtime player that loads the native library.

Mobile code follows the rules for applications and bindings in the `abra-library` skill's [applications document](../../abra-library/references/applications.md): the library does the work and the app is a thin caller, and these bindings expose only what the library has. In this binding layer a library tuple is an array and bad input panics, as the library does.

### Building the bindings

To build the Android bindings, run the following command from the root of the repository:

```bash
# Builds the Alakazam mobile bindings for Android
npm run build-bindings:android
```
