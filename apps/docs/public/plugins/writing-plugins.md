---
title: Writing a plugin
order: 1
outline: deep
---

# Writing a plugin

A plugin is a folder described by `plugin.yml`. Its `id` is its identity; its source folder name can be different. For the Alakazam app, TypeScript plugin sources live in `apps/alakazam/mobile/plugins/<folder>`. The build bundles their source into the files the app downloads. AI model plugins live under `packages/ai/<crate>/plugins/<folder>`.

```text
apps/alakazam/mobile/plugins/my-control/
  plugin.yml
  tsconfig.json
  src/
    control.ts

packages/ai/<crate>/plugins/ai-example/
  plugin.yml
  example.onnx
  example.yml
```

An installed plugin has its manifest and downloaded files together in the app's plugins folder:

```text
plugins/
  ai-lama-object-removal/
    plugin.yml
    lama_fp32.onnx
    lama_fp32.yml
```

## plugin.yml

| Field          | Required | Meaning                                                                                                          |
| -------------- | -------- | ---------------------------------------------------------------------------------------------------------------- |
| `id`           | yes      | Names the plugin: lowercase words of letters and digits joined by dashes. No two plugins share one.              |
| `name`         | yes      | The name to show.                                                                                                |
| `version`      | yes      | The version, such as `1.0.0`.                                                                                    |
| `description`  | no       | What the plugin does.                                                                                            |
| `author`       | no       | Who made the plugin, as the store page shows it. Without it the page shows the author part of the `id`.          |
| `store`        | no       | What the store page shows: `screenshots` (image files) and `tags`. Nothing here changes how the plugin runs.     |
| `assets`       | no       | Extra files to ship, such as sprites: files, folders (`sprites/`) or globs (`sounds/*.ogg`). See [Assets](#assets).  |
| `contributes`  | yes      | What it adds: `models`, `pipelines`, `scripts`, `controls` or `workspaces`. At least one contribution is needed. |
| `settings`     | no       | An ordered list of setting fields. Supports `text` and single-select `option`.                                   |
| `dependencies` | no       | The plugins that must be installed for this one to be used. See [Dependencies](#dependencies).                   |

### Settings

Settings are optional. When present, they are an ordered list of fields with a stable `id` and a visible `label`. `description`, `default`, and `required` are optional. Defaults and option values are text for now.

```yaml
settings:
  - type: text
    id: api-token
    attributes:
      label: API token
      description: Token used to access the service.
      placeholder: Enter a token
      value: ''
    validations:
      required: true
  - type: option
    id: quality
    attributes:
      label: Quality
      options: [fast, balanced, best]
      default: balanced
    validations:
      required: true
```

Each field has an `id`, a `type`, and `attributes` with a required `label` and optional `description`. A `text` field shows a text input and can have a `placeholder` and initial `value`. An `option` shows a single-selection list and needs a non-empty `options` list; its optional `default` must match one of those options. `validations.required` defaults to `false`.

Each model, pipeline, or script contribution has an `id` (the same rules, and unique in the plugin), a `title`, an optional `description`, and `capabilities`: a list of at least one word that a feature asks for, such as `inpaint`. A contribution with no capability could not be found. Controls instead use a root, a visible path, and a script entry, as described below.

Files are written relative to the plugin's folder, with `/` between names. A path may not start with `/`, hold `..`, or hold a drive.

### Models

An AI model that the AI runtime runs from its manifest.

| Field      | Required | Meaning                                                                 |
| ---------- | -------- | ----------------------------------------------------------------------- |
| `model`    | yes      | The `.onnx` file.                                                       |
| `manifest` | no       | The model's manifest. It is the model's name with `.yml` when left out. |
| `subjects` | no       | What the model finds, such as `people`, for showing it.                 |

The model's manifest says how the model is fed and what it answers with. It can describe every input tensor (a picture, a mask, a constant, as numbers or bytes), run the model on a window round a mask, and paste only the masked pixels back, so most models need no code at all. See `abra_ai_core::graph` for every setting.

### Pipelines

A list of library operations run in order.

```yaml
pipelines:
  - id: soften
    title: Soften
    capabilities: [effect]
    steps:
      - { op: gaussian_blur, radius: 12 }
      - { op: saturation, amount: 0.2 }
```

Each step has an `op`, which is the library operation as the host knows it, and its settings. A setting is a number, text, `true` or `false`, or a list of numbers.

### Scripts

A TypeScript module for logic that is more than a list of steps, run by the host's JavaScript engine against the same bindings the app itself uses.

```yaml
scripts:
  - id: collage
    title: Collage
    capabilities: [generator]
    entry: collage.js
```

### Workspaces

A workspace opens a full-screen plugin editor. Its JavaScript entry may export any of these, and the editor shows only the parts it exports:

| Export | What it does |
|---|---|
| `inspectorGUI()` | Adds the controls of the inspector beside the preview. Without it there is no inspector and no toolbar button for it. |
| `controlsGUI()` | Adds the controls of the bar under the preview, with the same functions as `inspectorGUI`. Without it there is no bar. |
| `previewCanvas()` | Returns the canvas the preview shows, or nothing, or a promise of it. A canvas that takes a while to make reads its pictures with `Image.readAsync` (below), which the library runs off the JS thread, never with a loop of image work on it; while the promise is pending the canvas on show stays, with a spinner over it, and only the newest call's canvas is shown. The editor shows it directly and the engine composes it on its own thread, so nothing is drawn on the JS thread. Called when the workspace opens and after each change that is not a drag. Return the same canvas again while it is still right, and change its layers in place (in a slider's `onChange`) to move something. |
| `dispose()` | Called when the workspace closes, so the plugin can free what it made, such as the canvas. |

To build a canvas of pictures, read each file once with `await Image.readAsync(path)` (a promise, decoded off the JS thread, so `Promise.all` reads several in parallel). Give each picture a cell: a blank canvas of the cell's size, holding a canvas of the picture's own size that you add with `addCanvas` and scale with `setScale` to cover or fit the cell (a canvas keeps its own pixels, so scaling never loses quality). Put the cells in the collage canvas with `addCanvas(cell, { position: [x, y] })`. The cells are the collage canvas's `canvases()`, which a slider can turn, scale and move with `setRotation`, `setScale` and `setPosition`.

The plugin owns the canvas it returns: free it (with `release`) when it returns another one, and in `dispose`. To add the native image picker to the inspector:

```ts
import { inspector } from '@alakazam/sdk';

export function inspectorGUI() {
  inspector.filePicker('Photos', {
    multiple: true,
    thumbs: true,
    onPick(files) {
      // `files` always has an array of copied local file URIs.
    },
  });
}
```

`source` is where the user picks from: `'file-system'` (the default) opens the device's file chooser, `'projects'` shows the saved projects and picks each one's edited image (its original if it was never edited), and `'both'` shows the projects with a button to browse the device's files instead. `multiple` defaults to `false`. `thumbs` defaults to `false`; when `true` the inspector shows a thumbnail of each image the user picked below the control; tapping one opens it in a full-screen viewer that swipes through all of them, and pressing and holding one starts selecting images to remove. The callback always receives an array of copied local files: the full list of images, called again with the images that are left when the user removes some, so keep the list it gives you. Each call to `inspectorGUI` or `controlsGUI` replaces the controls it made before, and the editor calls them again after a change that is not a drag.

To add a heading, like an `<h1>` to `<h6>`: `inspector.header('Options')` is an `h1`, and `inspector.header('Options', { level: 2 })` is an `h2`. The `level` option is 1 to 6 and defaults to 1.

To fold controls away, start a group: `inspector.group('Motion')` adds a title with an arrow, and every control added after it, up to the next `inspector.group`, is inside it. Tapping the title shows or hides them. Controls added before the first group are always showing. A group starts folded; pass `{ open: true }` to start it unfolded. Once the user has tapped a group their choice stays while the editor is open, even though the controls are made again after each change.

To let a group be switched on and off as a whole, pass `enabled` and `onToggle`: `inspector.group('Trail', { enabled: trailOn, onToggle: on => (trailOn = on) })` puts a checkbox before the title, ticked when `enabled` is `true`. The arrow still only folds the group. The editor runs the plugin again after the checkbox is tapped, and what the group's controls change is still kept while it is off, so the plugin decides what an off group means.

```ts
inspector.slider('Opacity', { max: 100 }); // always showing
inspector.group('Motion');
inspector.slider('Speed', { max: 10 });
inspector.group('Color', { open: true });
inspector.color('Tint');
```

To add two number fields side by side, such as a size or a point:

```ts
inspector.size('Image Size', {
  value: [1024, 1024],
  labels: ['Width', 'Height'],
  min: 1,
  onChange(value) {
    // `value` is `[width, height]`. The editor runs `previewCanvas` again after this.
  },
});
```

`value` defaults to `[0, 0]` and `labels` to `['Width', 'Height']`. `reset` (default `true`) shows a round reset button beside the name, which puts the numbers back to what they were when the editor opened. `min` and `max` are optional and limit both numbers. `onChange` is called with both numbers each time either field holds a number; a field that is empty or not a number changes nothing. Because a size changes what the plugin makes and is not a drag, the editor runs the plugin's controls and `previewCanvas` again after each change.

To let the person pick one of a few choices, as a bar of buttons across the width, and to add a single number:

```ts
inspector.select('Style', {
  options: ['Grid', 'Layered Grid', 'Random'],
  value: 'Grid',
  onChange(choice) {
    // `choice` is the button that was pressed.
  },
});

inspector.number('Images', {
  value: 6,
  min: 1,
  max: 100,
  onChange(count) {
    // `count` is held to `min` and `max`.
  },
});
```

`select` takes `options` (the choices) and `value` (the one selected, the first by default). `number` takes `value` (`min`, or 0, by default), `min`, `max`, `step` (1 by default, which is also how many decimals it shows) and `reset` (true by default). Both change what the plugin makes and are not drags, so the editor runs the plugin's controls and `previewCanvas` again after each change, the way it does for `size`.

To add an on/off switch:

```ts
inspector.toggle('Keep Aspect Ratio', {
  value: true,
  onChange(on) {
    // `on` is whether the switch is now on.
  },
});
```

`toggle` takes `value` (false by default) and `onChange`. Like `select` and `number` it changes what the plugin makes and is not a drag, so the editor runs the plugin's controls and `previewCanvas` again after each change.

#### Color

`color` shows a swatch of a color with its web color beside it, and opens the app's color picker when it is tapped:

```ts
inspector.color('Glow', {
  value: [255, 170, 40],
  onChange: ([red, green, blue]) => {
    glow = [red, green, blue];
  },
});
```

The picker has a square to drag a finger over for how colorful and how bright the color is, a bar for which color it is, the new color beside the one it started as (tap the current one to go back to it), named colors for the common choices, and boxes to type the color as numbers or a web color. `value` is `[red, green, blue]`, each from 0 to 255, and white by default. With `alpha: true` the value has a fourth number, the strength from 0 to 255, and the picker has a Strength bar. `title` is the words over the picker (the control's name by default) and `swatches` is a list of `{ label, rgb: [r, g, b] }` for the named colors it shows (a common set by default), so a plugin can offer the colors that suit what it makes. `onChange` gets the color when the person presses OK, and the editor runs the plugin again after it.

`onPreview` gets the color as the person drags in the picker, before OK, so the plugin can show it at once, and it is called once more with the color the control had if the picker is cancelled, so the change is taken back. The editor does not run the plugin again for it, so change what is showing (as a slider's `onChange` does), such as a color in an animation that is already playing.

To ask for a color anywhere else in a script, `app.colors.pick({ title, value, alpha, swatches, onPreview })` opens the same picker and answers with the color on OK (in the same form) or `null` on Cancel.

#### Gradient

`gradient` shows a bar of a gradient (over a checkerboard, so how strong it is shows), and opens the gradient editor when it is tapped:

```ts
inspector.gradient('Color Over Life', {
  value: {
    colors: [{ at: 0, color: [255, 240, 160] }, { at: 1, color: [220, 40, 10] }],
    alphas: [{ at: 0, alpha: 255 }, { at: 1, alpha: 0 }],
  },
  onChange: value => {
    fire = value;
  },
});
```

The editor shows the gradient as a bar with the strength stops above it and the color stops below it, as in Unity. Drag a stop to move it, touch an empty place to add one, and tap a stop to set its color (it opens the color picker), its strength or its location, or to remove it. `at` is from 0 (the start) to 1 (the end), `alpha` from 0 (not there) to 255 (solid), and each list needs at least one stop. `title` and `swatches` are as for `color`, and `onChange` and `onPreview` work the same way (`onChange` on OK, and the editor runs the plugin again after it; `onPreview` as the gradient is edited, and once more with the old gradient on Cancel).

#### Curve

`curve` shows a small drawing of a curve across the row, and opens the curve editor when it is tapped:

```ts
inspector.curve('Size Over Life', {
  value: [[0, 0.2], [1, 1]],
  presets: [
    { label: 'Grow', value: [[0, 0.2], [1, 1]] },
    { label: 'Shrink', value: [[0, 1], [1, 0.2]] },
  ],
  onChange: points => {
    size = points;
  },
});
```

The editor draws the curve on a grid with its points to drag. Touch an empty place to add a point, and tap a point to set its time and value or to remove it. The curve is a smooth line through the points, drawn by the library's own curve, so it is the line your plugin makes when it builds a `Curve` from the same points (the library's `x` runs from 0 to 255, so multiply the first number of each point by 255). Each point is `[x, y]` with `x` from 0 (the start) to 1 (the end) and `y` from `min` to `max` (0 and 1 by default, which is also the range of the editor). The curve needs a point, and is flat before the first point and after the last. `presets` are named shapes to start from, `title` is the words over the editor, and `onChange` and `onPreview` work as for `gradient`.

To add a box for a line of text:

```ts
inspector.text('Text', {
  value: 'Summer 2026',
  placeholder: 'Write something',
  onChange(text) {
    // Called a moment after typing stops.
  },
});
```

`text` takes `value` (nothing by default), `placeholder` and `onChange`, which is called a moment after typing stops, not for every letter. The editor then runs the plugin's controls and `previewCanvas` again.

To let the person choose a font:

```ts
inspector.fontPicker('Font', {
  value: 'Roboto',
  onPick(family) {
    // `family` works with app.fonts.renderText.
  },
});
```

`fontPicker` shows a button with the chosen family (`value`, or "Choose a font"). Tapping it opens the Fonts overlay, where the person browses the fonts on their device and downloads more from Google Fonts. `onPick` gets the family they tap, and the editor then runs the plugin's controls and `previewCanvas` again.

To write text on a picture, use the fonts the person downloaded on the Fonts screen. `app.fonts.installed()` lists their family names, and `app.fonts.renderText({ text, family, size, color, width, align })` gives back a new transparent image of the text, which you release when done. It gives back `null` when there is nothing to write (no text, or a family that was deleted). The editor writes text with the same function.

To let people pick a thing on the preview and edit it, the way Unity and Godot show the settings of the selected object, keep the pick in the plugin and make `inspectorGUI` show different controls for it:

```ts
let selected: number | null = null;

export const inspectorGUI = () => {
  if (selected === null) return mainControls();
  inspector.header('Image ' + (selected + 1), { back: () => pick(null) });
  // ...the controls of the picked image
};

const pick = (index: number | null) => {
  selected = index;
  inspector.refresh(); // runs inspectorGUI and previewCanvas again
};

app.events.taps.subscribe(({ imageX, imageY }) => {
  const found = imageX === null || imageY === null ? undefined : canvas.childAt(imageX, imageY);
  // ...find which of your canvases it is (compare `id()`), then pick(index)
});
```

`header` takes `back`, which shows a Back button with the heading, and `inspector.refresh()` runs `inspectorGUI` and `previewCanvas` again for a change that did not come from one of the controls (a tap on the preview). `canvas.childAt(x, y)` gives the canvas on top at a point (in the canvas's own pixels), and `canvas.quadInParent()` gives the four corners of a canvas as it sits in its parent, after its position, scale and rotation. `app.events.taps` gives `imageX` and `imageY` in the pixels of the canvas `previewCanvas` returned. `canvas.setViewInFrame(width, height, { zoom, turn, pan })` zooms, turns and moves a picture canvas inside a frame it has to cover (a photo in a cell): a zoom of 1 just covers the frame, and the library keeps it from showing empty corners at any turn or move, so a pinch only has to pass the numbers on.

To show what is picked, draw a frame with a gizmo:

```ts
const corners = quadCorners(cell.quadInParent()).map(corner => [corner.x, corner.y]);
app.gizmos.setBox('selection', corners);

const red = Color.red();
app.gizmos.color(red); // the gizmos drawn from here on are red
app.gizmos.setBox('problem', otherCorners);
app.gizmos.color(null); // back to white, the default
release(red);

// later: app.gizmos.remove('selection');
```

`app.gizmos.setBox(name, corners)` draws a frame round four corners (top left, top right, bottom right, bottom left) in pixels of the canvas, and the app keeps it on the picture as the preview is zoomed, moved and turned. It is drawn in the color set with `app.gizmos.color(color)`, which stays until it is set again (`null` goes back to white), and as wide as `app.gizmos.size(pixels)` says (3 screen pixels by default). A gizmo that is already drawn keeps the color it was drawn in. A soft dark line is drawn under each frame so it shows on any picture. Setting a name again moves it, and `app.gizmos.remove(name)` takes it away. They go away when the workspace closes. Move a frame again when what it frames moves (a slider that turns the cell).

`app.gizmos.setPoint(name, [x, y])` draws a dot at a point given as shares of the canvas: `[0, 0]` is its top left and `[1, 1]` its bottom right. The dot has a dark halo, a white ring and the gizmo color in the middle, stays the same size on the screen at any zoom, and follows the picture. `app.gizmos.pickPoint(name, onPick)` lets the person tap the preview to choose a point: each tap on the canvas calls `onPick` with the point in the same shares and puts the dot there (a tap off the canvas does nothing), and it stops when the workspace closes. Use it for any point the person chooses, such as where an effect comes from or where a gradient ends:

```ts
app.gizmos.pickPoint('source', point => {
  source = point;
  inspector.refresh(); // makes the preview again
});
```

To add a slider:

```ts
inspector.slider('Rotation', {
  min: -360,
  max: 360,
  step: 1,
  range: true,
  onChange(value) {
    // `value` is `[low, high]` here, because `range` is `true`.
  },
});
```

`max` is required. `min` defaults to `0`, `step` to `1` and `range` to `false`. `multiline` puts the name on a line of its own with the slider under it across the whole width; otherwise the name is on the left and the slider on the right. It defaults to `false`, and to `true` when `range` is `true`, since two thumbs need the width. A number field is at the end of the slider (at both ends for a range) so a value can be typed. `reset` (default `true`) shows a round reset button beside the name, which puts the slider back to where it was when the editor opened. With `range: false` the slider has one thumb and `onChange` gets a number; with `range: true` it has two thumbs and `onChange` gets `[low, high]`. `value` is where it starts: `min` by default (`[min, max]` for a range). `onChange` hears every move of a drag, and the preview is drawn again once a frame; `previewCanvas` is not called again, so change the layers of the canvas it returned. Set `rebuild: true` for a value that changes how the picture is made (a size, where resizing a finished piece would blur it): `previewCanvas` is then called on each move.

Declare the workspace entry in `plugin.yml`:

```yaml
workspaces:
  - id: collage
    title: Collage
    entry: collage.js
```

### Controls

A control for the editor: a slider, with the code that paints it. The plugin says where its controls go and what they are called, and a small script makes each one.

```yaml
contributes:
  controls:
    - root: Beauty/Makeup # the tab, then the groups inside it, by the names people see
      items:
        - path: Lipstick # the control's name, under the root
          entry: lipstick.js
        - path: Eyes/Eyeliner # a group Eyes is made in Makeup, and Eyeliner goes in it
          entry: eyeliner.js
    - root: Beauty/Hair # a plugin can have more than one root
      items:
        - path: Hair Color
          entry: hair-color.js
```

- **The root** is matched to the names in the app, ignoring case. A tab or group that is not there is made, so a plugin can add a tab of its own. If the plugin is taken away, a tab or group it made goes with it once it is empty.
- **The path** is the control's name, which is what people see, and so the script does not need to say it. A path with more names puts the control in groups under the root.
- **The order** is the order written here.
- **The script** is TypeScript in `src/` named like its entry (`src/lipstick.ts` builds to `lipstick.js`). Its default export is a function that returns the control, in the same shape the app's own controls use. It can import the Abra library (`@abra/core/...`), `@alakazam/sdk` (what the app offers plugins: the face masks, the layer painter and a few helpers), and other files of the plugin by relative path, which are built into the same file.
- **Types** in app-owned plugins are checked with the app's plugin config. The public npm builder does not require the Alakazam app source: it transpiles TypeScript and leaves documented runtime imports external. Plugin authors can add their own type-checking setup if they want one.

## Build and package

For a plugin with TypeScript controls or scripts, put its source in `src/` beside `plugin.yml`. The npm build tool bundles its TypeScript entries and relative imports without needing the Alakazam app source code.

Each source file matches its manifest entry: `entry: color.js` is built from `src/color.ts`. Relative imports are bundled into the entry. Imports such as `@abra/core/color` and `@alakazam/sdk` stay external for Alakazam to provide at runtime.

Build from the plugin folder, or pass the folder path:

```sh
npx @alakazam/sdk build
# or
npx @alakazam/sdk build ./my-plugin
```

Add `@alakazam/sdk` and `@abra/core` to your plugin's `devDependencies` to type-check host API imports in your own TypeScript project. The package provides the declarations for `@alakazam/sdk`; Alakazam supplies its runtime implementation when the plugin loads.

The build updates the `files:` list and SHA-256 hashes in `plugin.yml`, then writes `dist/<id>.akzx`. An `.akzx` package is a ZIP archive. Plugin authors can install it from **Plugins → Local plugin package → Choose .akzx** without uploading to S3. The local installer verifies every file against its manifest hash. Declared dependencies still need to be installed before the plugin can be used.

In this repository, `node scripts/build-plugins.mjs <folder>` delegates to the same npm builder and also type-checks app-owned plugins. `scripts/pack-plugins.mjs [folder ...]` is for maintainers preparing catalog uploads; plugin authors do not need it. It checks files and assembles upload folders under `target/plugin-packages/`, including model files from the AI crates when needed. Each upload folder contains exactly `plugin.yml` and `<id>.akzx`; the catalog entry has `uri`/`sha256` for the manifest and `archive_uri`/`archive_sha256` for the `.akzx` package. The app reads the manifest for metadata and dependencies, then installs the package after verifying both hashes and each file hash in the manifest.

A plugin script runs with all of the app's rights. Install only packages from authors you trust. File hashes detect a damaged or mismatched package; they do not identify who made it.

## Assets

List the extra files a plugin needs (sprites, sounds, data) under `assets:`. Each entry is a file, a folder (everything under it) or a glob, relative to the plugin's folder:

```yaml
assets:
  - sprites/
  - sounds/*.ogg
  - noise.png
```

The build expands the entries, hashes every file it finds and adds them to `files:`, so they are packaged, installed and synced by the dev server with the rest of the plugin. An entry that matches no file fails the build. In a script, `app.assets.path('sprites/spark.png')` from `@alakazam/sdk` gives the path of an asset on the device; it throws for a file the plugin did not ship.

## Saves

A plugin that makes things for other projects to use later (a particle maker's systems, say) keeps them with `app.saves` from `@alakazam/sdk`. A save is named text, so write JSON:

```ts
import { app } from '@alakazam/sdk';

app.saves.write('fire.json', JSON.stringify(system));
app.saves.list(); // ['fire.json', 'snow.json']
const text = app.saves.read('fire.json'); // undefined when there is no such save
app.saves.remove('fire.json');
```

Saves are shared by every project and are the plugin's own: a plugin cannot reach another's. They are kept beside the plugins and not inside the plugin, so removing a plugin keeps what it saved, and installing it again finds the saves where they were. A name is letters, digits, spaces, `-`, `_` and `.` (at most 100, and it cannot start with a dot), a save is at most 5 MB, and each function throws for a name that is not allowed or a write the device cannot do. Use `app.saved` instead for what belongs only to one project.

## Numbers that are not real

A number that is NaN or infinite is easy to make (a field a save did not have, a division by zero) and hard to find: it goes on to the library as a radius or a size, and the picture is just blank. The editor guards the edges for every plugin, so a plugin does not have to remember to:

- **Controls:** a number a plugin gives a control (`value`, `min`, `max`, `step`, a color channel) that is not real is replaced (the lowest value for a slider or number, white for a color) and said in the log, once, with the plugin's id. A slider whose `max` is not above its `min` gets a range of one. A step that is not above 0 is 1.
- **Callbacks:** the editor never calls `onChange` with a number that is not real. A slider, number or size field ignores a value that is NaN or infinite, so a plugin's `onChange` only ever sees real numbers.
- **Saved data:** `app.saved.set` says in the log when what it was given holds a number that is not real, because JSON writes it as `null` and it is read back as nothing. The editor cannot fix that for you, since it does not know what the value is for, so check what you read back from `app.saved` and `app.saves`.
- **Sprites:** the library's `Sprites` effect leaves out a sprite or trail that has a number that is not real, so one bad value draws nothing and does not put a dot at the corner of the picture.

## README and changelog

A plugin can have a `README.md` and a `CHANGELOG.md` in its folder, next to `plugin.yml`. Both are optional. When the build finds them it adds them to `files:` with their hashes, so they are packaged and installed with the plugin. The app finds them through `readme_path` and `changelog_path` on the plugin returned by `scanPlugins`; each is empty when the plugin does not ship that file.

## Dependencies

A plugin can need other plugins. A plugin of controls that use AI models, for example, does not carry the models: it names the plugin that does.

```yaml
dependencies:
  - id: alakazam.face-landmarks
  - id: alakazam.face-parsing
```

Each plugin `id` uses `<author>.<id>` so plugins from different authors can use the same name. The app's `catalog.yml` lists every plugin that can be installed, and the app looks dependencies up there by their full IDs.

- **Installing** a plugin installs the plugins it depends on that are not on the device yet, and what those depend on, before it. If one of them is not in the catalog, nothing is installed and the app says which.
- **Using** a plugin needs every plugin it depends on to be installed and usable itself. Until then the plugin is listed, with the missing ones named, but it adds nothing. Plugins that depend on each other in a circle are never usable.
- A plugin cannot depend on itself or list the same dependency twice. A dependency has only an `id`, and another key is an error, so a misspelled one is not silently ignored.

## How an app uses a plugin

An app never names a plugin. A feature asks for a capability, and offers each contribution that provides it. The Remove Objects tool, for example, asks for `inpaint`, so adding another plugin with that capability adds another choice to the tool without a change to the app.

## When a plugin does not load

`PluginRegistry::scan` reports each plugin it could not use in `problems()` with a message for a person, such as the file a plugin names that is not there. The other plugins load as usual.
