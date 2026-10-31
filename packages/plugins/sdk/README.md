# @alakazam/sdk

Build and package an Alakazam plugin from its folder:

```sh
npx @alakazam/sdk build ./my-plugin
```

The folder must contain `plugin.yml` and a `src/` directory. Entries in the manifest that end in `.js` are built from matching `.ts` files in `src/`; relative imports and statically imported npm dependencies are bundled into each entry. Alakazam does not install the plugin project's `node_modules` or resolve arbitrary npm package names at runtime. Its plugin loader provides `@alakazam/sdk` and supported `@abra/core` entry points; other package code must be bundled into the entry. Bundling does not guarantee runtime compatibility or add missing APIs, so test third-party packages in Alakazam on Android. Packages that require Node-only modules such as `crypto` may not work in Alakazam's JavaScript runtime. The tool updates the manifest's `files:` hashes and writes `<id>.akzx` into `dist/`. The `.akzx` package is a ZIP archive containing `plugin.yml` and every file listed by the manifest, ready to install from Alakazam's Plugins screen.

Run the build command from the plugin folder with no argument, or pass one or more plugin folders to build them.

For Android development, start **Plugin development server** on the app's Plugins page. Keep the phone and computer on the same network, then run:

```sh
npx @alakazam/sdk dev
```

The command prompts for the server address shown in the app. Pass `--url <address>` to provide it directly. By default, it searches the workspace repository root for plugin folders with JavaScript controls, including when npm launches it from the toolkit package directory. Pass `--root <directory>` or positional search roots to search elsewhere. At startup it only watches; saving a plugin file rebuilds the entry and sends `plugin.yml` and its listed files directly to the app. It does not create an archive for live sync. The app validates and installs the files, then refreshes its controls. Stop the server from the Plugins page when finished.

The package also provides the TypeScript declarations for `@alakazam/sdk`, the module Alakazam exposes to plugins at runtime. Add `@alakazam/sdk` and `@abra/core` to your plugin's development dependencies to type-check its imports.

Plugin controls can temporarily change the AI pending notice while doing work:

```ts
import { setAiPendingNotice } from '@alakazam/sdk';

setAiPendingNotice({ text: 'Working on your edit', detail: 'Preparing the selected area' });
try {
  // Do the plugin work.
} finally {
  setAiPendingNotice(null); // Restore the generic AI notice.
}
```

The runtime toolkit exposes generic app capabilities through `app`: `app.editor` provides the current image, drawn areas, and editor state; `app.ai` provides model selection, loading, inference, and progress; `app.history` commits and saves edits; and `app.tools` displays temporary tool feedback. These APIs are intended to let plugins connect their own domain logic to the app without putting plugin-specific behavior in the toolkit.

A tool control may provide `onApply` for an app-provided panel action and `panel` to select an app panel by its registered identifier. Plugins own their operation keys and behavior. For example, object removal uses the generic editor, AI, history, and feedback APIs while keeping the removal operation in the object-remover plugin.
