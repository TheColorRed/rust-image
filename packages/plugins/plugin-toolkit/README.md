# @alakazam/plugin-toolkit

Build and package an Alakazam plugin from its folder:

```sh
npx @alakazam/plugin-toolkit ./my-plugin
```

The folder must contain `plugin.yml` and a `src/` directory. Entries in the manifest that end in `.js` are built from matching `.ts` files in `src/`, with relative imports bundled into each entry. The tool updates the manifest's `files:` hashes and writes `<id>.akzx` into `dist/`. The `.akzx` package is a ZIP archive containing `plugin.yml` and every file listed by the manifest, ready to install from Alakazam's Plugins screen.

Run the command from the plugin folder with no argument, or pass more than one plugin folder to build several plugins.

The package also provides the TypeScript declarations for `@alakazam/plugin-toolkit`, the module Alakazam exposes to plugins at runtime. Add `@alakazam/plugin-toolkit` and `@abra/core` to your plugin's development dependencies to type-check its imports.

Plugin controls can temporarily change the AI pending notice while doing work:

```ts
import { setAiPendingNotice } from '@alakazam/plugin-toolkit';

setAiPendingNotice({ text: 'Working on your edit', detail: 'Preparing the selected area' });
try {
  // Do the plugin work.
} finally {
  setAiPendingNotice(null); // Restore the generic AI notice.
}
```

The runtime toolkit exposes generic app capabilities through `app`: `app.editor` provides the current image, drawn areas, and editor state; `app.ai` provides model selection, loading, inference, and progress; `app.history` commits and saves edits; and `app.tools` displays temporary tool feedback. These APIs are intended to let plugins connect their own domain logic to the app without putting plugin-specific behavior in the toolkit.

A tool control may provide `onApply` for an app-provided panel action and `panel` to select an app panel by its registered identifier. Plugins own their operation keys and behavior. For example, object removal uses the generic editor, AI, history, and feedback APIs while keeping the removal operation in the object-remover plugin.
