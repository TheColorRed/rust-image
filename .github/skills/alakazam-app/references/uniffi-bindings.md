# Bindings

## Library integration/bindings

The mobile app in `/apps/mobile` uses the Abra library for image manipulation through the `@alakazam/mobile` package in `/packages/react-native/alakazam`. The package's bindings are generated with `uniffi-bindgen-react-native` using its `jsi2` flavour. They include the `libalakazam_mobile.so` native library, generated TypeScript files, Android integration files, and a CocoaPods specification. `apps/mobile` depends directly on `@ubjs/react-native` so React Native autolinking installs the runtime player that loads the native library.

### Building the bindings

To build the Android bindings, run the following command from the root of the repository:

```bash
# Builds the Alakazam mobile bindings for Android
npm run build-bindings:android
```
