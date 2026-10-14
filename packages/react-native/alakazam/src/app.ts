// The entry of `@alakazam/mobile`: the app's own native module (the preview, the history, the model downloads).
//
// The bindings generator writes `generated/`, whose barrel (`generated/index.ts`) also exports the Abra library, which would
// make every library name importable from two places. This entry exports only the app's namespace; the library is
// `@abra/core`. The generator's own entry file is written to `.ubrn/` (see ubrn.config.yaml) and nothing imports it: this file
// does what it did instead.

// The player must be a direct dependency of the app: autolinking reads only the app's package.json, so a transitive
// @ubjs/react-native never installs.
import '@ubjs/react-native';
// Starts every native module, the library's included: this must run after the player is imported and before any of them
// opens the library.
import './generated';

if (typeof (globalThis as any).uniffi === 'undefined') {
  throw new Error('@ubjs/react-native must be a direct dependency of the app; add it and re-run pod install');
}

export * from './generated/alakazam_mobile';
// The editor's image is the shared Abra UniFFI image. These aliases preserve the mobile package's public import path while
// avoiding a second native image wrapper.
export { Image as AbraImage } from './generated/abra_uniffi';
export type { ImageLike as AbraImageLike } from './generated/abra_uniffi';
