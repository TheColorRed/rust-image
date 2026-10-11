// The Abra library as a tool file sees it: `import { Color, blend } from '@abra/core'`.
//
// The library's types and functions come from the `abra-uniffi` crate (packages/bindings/uniffi), which is one namespace of
// the bindings that `@alakazam/mobile` generates (`src/generated`). This re-exports it whole, so a new function in that crate
// appears here once the bindings are built. The app's own native module (the preview, the history, the model downloads)
// stays in `@alakazam/mobile`.

// Starts the native modules before the library's functions are used.
import '@alakazam/mobile';

// export * from '../alakazam/src/generated/abra_uniffi';
export {};
