# Develop an Alakazam plugin

You can develop an add-on in your own project without the Alakazam source repository. For live testing, the toolkit watches your files and syncs changes to Alakazam over your local network. Build an `.akzx` package when you want to share or manually install the plugin.

## What you need

- Node.js 20 or newer and npm.
- The `@alakazam/plugin` and `@abra/core` npm packages.
- For live testing: Alakazam on Android and a local network connection shared with your computer.
- For manual installation: Alakazam on your phone.

## Create a plugin project

Make a folder for the plugin and install its development dependencies:

```sh
mkdir example-plugin
cd example-plugin
npm init -y
npm install --save-dev @alakazam/plugin @abra/core
mkdir src
```

Create a `plugin.yml` in the project folder. The plugin ID is a lowercase author name and plugin name separated by a dot. Use your own unique ID for publication.

```yaml
$schema: https://alakazam-image-editor.s3.us-east-1.amazonaws.com/plugins/plugin.schema.json
id: your-name.example-addon
name: Example Add-on
version: 0.1.0
description: An example editor add-on.
contributes:
  controls:
    - root: Effects
      items:
        - path: Example
          entry: example.js
```

The `entry` names the built JavaScript file. Its matching TypeScript source goes in `src/`, so this entry is built from `src/example.ts`:

```text
example-plugin/
  package.json
  plugin.yml
  src/
    example.ts
```

Create `src/example.ts` and export a function that returns an editor control:

```ts
import { saturation } from '@abra/core/adjustments';
import { toolKey, type SliderControl } from '@alakazam/plugin';

const example: SliderControl<number> = {
  kind: 'slider',
  key: toolKey('colorfulness'),
  label: 'Colorfulness',
  min: -100,
  max: 100,
  step: 1,
  defaultValue: 0,
  tool(photo, values, _context) {
    saturation(values[0]).apply(photo);
  },
  thumb(photo, _context) {
    return saturation(100).applied(photo);
  },
};

export default () => example;
```

`toolKey('colorfulness')` adds the current plugin ID to the key. For a plugin with ID `your-name.example-addon`, it returns `your-name.example-addon.colorfulness`. This keeps the key unique and gives it the prefix Alakazam requires.

The slider's `tool(photo, values, context)` callback receives three parameters:

- `photo` is the working image. Change it in place, or return an image to use as the result.
- `values` is an array of the slider values, in control order. This one-slider example reads `values[0]`; a control with several sliders can use more entries.
- `context` provides extra information when available, such as the preview scale, preloaded plugin data, selected person, and drawn areas. It can be ignored when the callback does not need it.

The optional `thumb(photo, context)` callback receives a thumbnail-sized working image and the same kind of context. Return the preview image or change `photo` in place. Here it shows maximum saturation; omit `thumb` if a preview is not useful.

## Develop and test from your computer

Keep the plugin project on your computer. For live testing, your Android phone and computer need to be on the same Wi-Fi network.

1. In Alakazam, open **Plugins** and turn on **Plugin development server** in the developer card at the bottom of the page. Keep Alakazam open. The app shows the address to use.
2. In a terminal, change to the folder containing your `plugin.yml` and run:

   ```sh
   npx @alakazam/plugin dev
   ```

3. Enter the address shown in Alakazam when prompted. You can provide it on the command line instead:

   ```sh
   npx @alakazam/plugin dev --url 192.168.1.20:43821
   ```

4. Wait for `Watching 1 plugin folder(s)` in the terminal, then edit and save a plugin file such as `src/example.ts`. The command searches the current folder tree for `plugin.yml` files and builds the matching plugin when you save a relevant file. It sends `plugin.yml` and the plugin's listed files directly to Alakazam; the app validates and installs the update, then refreshes its controls.

The command does not build or install anything when it starts. After it says it is watching, save a plugin file once to trigger the first build and sync. Live sync sends the raw files and does not create an `.akzx` archive. Keep Alakazam open while developing, and turn off the development server from the Plugins page when you finish. Live sync updates JavaScript add-ons; install any model plugins separately.

If your terminal is not in the plugin project, pass its folder to the command, for example `npx @alakazam/plugin dev ./example-plugin`. You can also pass `--root <directory>` to set the folder tree to search.

## Use an npm package in your plugin

The app can run npm package code only when it is bundled into your plugin's JavaScript entry. Install the package in your plugin project:

```sh
npm install date-fns
```

Import it from the TypeScript source for an entry in `plugin.yml`:

```ts
import { format } from 'date-fns';

const labelForToday = () => format(new Date(), 'yyyy-MM-dd');
```

You do not need a separate bundler or a `vendors.js` file. The toolkit uses esbuild to bundle static npm imports directly into `dist/example.js`. When you save the source with `npx @alakazam/plugin dev` running, it also adds `example.js` and its generated hash to `plugin.yml`, then syncs those files to the app. The app runs the bundled entry; it does not receive `node_modules`.

The `dependencies` field in `plugin.yml` is for other Alakazam plugins, not npm packages. Packages that rely on Node APIs or native Node add-ons may not work in Alakazam; test them in the app.

The toolkit provides TypeScript declarations. To type-check your source, install TypeScript and create a `tsconfig.json`:

```sh
npm install --save-dev typescript
```

```json
{
  "compilerOptions": {
    "target": "ES2019",
    "module": "commonjs",
    "moduleResolution": "node",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true
  },
  "include": ["src/**/*.ts"]
}
```

Then run `npx tsc --noEmit`.

For the full manifest format, control types, dependencies, and runtime APIs, see the [plugin writing reference](https://github.com/TheColorRed/rust-image/blob/main/apps/docs/public/plugins/writing-plugins.md).

## Package a plugin for sharing or manual installation

Live sync is for development. To make a package that can be installed from the regular Plugins screen, run this from your plugin project folder:

```sh
npx @alakazam/plugin build
```

The toolkit builds each JavaScript entry from its matching TypeScript file, writes output under `dist/`, updates the `files` list and SHA-256 hashes in `plugin.yml`, and creates `dist/your-name.example-addon.akzx`.

Copy the `.akzx` file to your phone, for example into **Downloads**. In Alakazam, open **Plugins**, choose **Install from file**, and select it. Installing a package with the same plugin ID replaces the installed version. See [Plugin management](/guide/plugins) for managing installed add-ons. If the plugin declares dependencies, install those plugins separately; an `.akzx` does not automatically include dependency plugins or AI model files.

## Troubleshooting

- **The package command cannot find `plugin.yml`:** Run it from the folder containing `plugin.yml`. If you are in its parent folder, pass the plugin folder as an argument: `npx @alakazam/plugin build ./example-plugin`.
- **The build says an entry is missing:** Check that each `.js` entry in `plugin.yml` has a matching `.ts` file under `src/`, with the same relative path.
- **The app reports that it does not provide an imported module:** The built entry still has an external import that is not in the app's module list. Use a static import so the toolkit can bundle third-party code, and check whether the package depends on unsupported Node APIs or dynamic loading.
- **Alakazam reports a missing dependency:** Install the dependency plugin separately, then reopen or reload the add-on.
- **The computer cannot connect:** Check that the phone and computer are on the same network, the server is running in the app, and you entered the matching address.
- **The app does not update:** Keep Alakazam open while the server is running. Check the app for an installation error, and confirm the plugin build succeeded in the terminal.
