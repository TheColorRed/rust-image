# Structure of the Alakazam Mobile App

The Alakazam mobile app is a React Native application in `/apps/alakazam/mobile`. Its image-manipulation bindings live in the `@alakazam/mobile` package in `/packages/react-native/alakazam`. Below is an overview of the main components:

```text
apps/alakazam/mobile/
├── package.json                    # React Native app configuration and scripts
├── app.json                        # Application name and display name
├── index.js                        # React Native application entry point
├── babel.config.js                 # Babel configuration
├── metro.config.js                 # Metro bundler configuration
├── global.css                      # Global NativeWind styles
├── tailwind.config.js              # Tailwind/NativeWind configuration
├── tsconfig.json                   # TypeScript configuration
├── android/                        # Android project and native integrations
├── assets/                         # Application assets
└── src/
    ├── App.tsx                     # Root application component and navigation state
    ├── components/                 # Reusable UI components
    │   └── tools/                  # Tool-specific UI components
    ├── hooks/                      # Custom React hooks
    ├── lib/                        # Theme, styling, media, and utility modules
    │   └── layers.ts               # The editor canvas: a root canvas with a child canvas per group (base, skin, makeup), each holding a layer per feature (see "The editor canvas")
    └── screens/                    # Home, editor, camera, settings, and Abra screens

packages/react-native/alakazam/
├── package.json                    # @alakazam/mobile package configuration
├── Cargo.toml                      # Rust crate configuration for the mobile bindings
├── ubrn.config.yaml                # uniffi-bindgen-react-native configuration
├── AlakazamMobile.podspec          # Generated iOS CocoaPods integration
├── android/                        # Generated Android React Native integration
├── scripts/
│   └── build-android.mjs           # Builds Android libraries and regenerates bindings
└── src/
    ├── lib.rs                      # UniFFI crate entry point
    ├── image.rs                    # Abra image binding API
    ├── adjustments.rs               # Image adjustment binding API
    ├── filters.rs                   # Image filter binding API
    ├── tools.rs                     # Image tool binding API
    ├── transforms.rs                # Image transform binding API
    ├── history.rs                   # Edit-history binding API
    ├── index.tsx                   # React Native binding package entry point
    └── generated/                  # Generated UniFFI TypeScript bindings
```

## The editor canvas

The edit is one root canvas the size of the photo, flattened with `asImage()` into the result. The photo itself is never changed. Each group of features (skin, makeup, ...) is a child canvas of the root, added with `addCanvas`, and each feature in a group is a layer of that child. Groups stack bottom to top in the order below, so makeup always sits over the skin edits.

```
root canvas  (photo-sized, flattened with asImage() into the result)
├── base canvas                  the untouched photo
│   └── photo layer
├── skin canvas
│   ├── tan layer                everyone: the adjusted skin, limited to the skin mask
│   ├── tan:person:2 layer       one person's own tan, limited to that person's skin
│   ├── tone layer
│   └── smooth layer
└── makeup canvas
    ├── eyeshadow layer          everyone: own blend + opacity, cut down to the eyes, setPosition(x, y)
    ├── eyeshadow:person:2 layer
    ├── blush layer
    └── lipstick layer           top: cut down to the lips
```

**Every feature can be set for one person.** A value is stored for everyone (`key`) or for one person (`key:person:<id>`, see `adjustmentKey`). Each person with their own value gets their own layer named `<feature>:person:<id>`, whose masks are cut down to that person (`personMasks`). The "everyone" layer leaves those people out (their masks are subtracted from its masks), so a person's own value replaces the shared one for them and the two never stack. A person with no value of their own is covered by the "everyone" layer. Layers are in the same group canvas, so the person layers stack in the same order as the group.

**Every group canvas must be pass-through** (`setPassThrough(true)`). Otherwise a group is flattened on its own and its layers blend against an empty canvas instead of the photo, so a Color or Multiply layer such as lipstick comes out as flat, stronger color. The layers of a pass-through group are composited straight into the root, in order, with their own blend modes.

**Status: this is the target structure. The code is not there yet.** `withLayers(photo, sheets)` in `src/lib/layers.ts` still builds one flat canvas (`base` plus the makeup sheets), and skin tools (tan, tone, smooth) still edit the photo in place with the skin mask instead of being layers. Rewrite both to match this tree, and delete this note when done. A layer is made by one control: a control with a `layer` paints a sheet, and a skin tool must produce its own layer.

- A sheet is only as big as the part of the photo that was painted (a cheek, the lips), and `setPosition(x, y)` puts it back. It is not photo-sized, because a photo-sized sheet costs as much memory as the photo.
- `paintSheet(painting, regions, color)` makes that cut-down sheet: `color` fills the union (`maximum`) of the `regions` masks, as strongly as the masks are at each pixel.
- A sheet that paints nothing is left out of the canvas.
- Every native object made while building the canvas is freed in a `finally`, so a failed paint leaks nothing.
- Skin tools (tan, tone, smooth) are not layers. They edit the photo in place with the skin mask, and the makeup sheets go over them. During a slider drag the steps that come after the slider are laid over the new result (`renderStepsAfterSlider`), which is why makeup stays visible while the skin changes.

### Masks the canvas tools use

`makeupMasks` (`src/lib/eye-makeup.ts`) finds each face and makes full-photo masks named `kind|box`, where `box` is the face's box. The kinds are `lipstick` (both lips), `mouth` (inside the mouth, so the teeth and tongue; the face-parsing model has no teeth label), `skin`, `nose`, `eye|…|left/right` and `brow|…|left/right`. A control receives them as `preloaded` by naming `preload: makeupMasks`, and they are kept with the project.

The whole-photo skin mask comes from `skinMaskOf` / `skinMaskSized` in `src/lib/analysis.ts`; the skin model counts part of the lips and mouth as skin. `skinWithMouthEased` (`src/lib/eye-makeup.ts`) gives the lips and mouth half of a skin edit with a soft edge, and the skin tan and skin tone tools use it.

## How data is stored on the phone

The app keeps nothing in a database, `AsyncStorage` or `MMKV`. What survives a restart is files in the app's private folders (Android). Everything else is in memory and is rebuilt on start.

| What | Where | Lasts until | Written by |
|---|---|---|---|
| Projects | `filesDir/projects/<id>/` | The user deletes the project, uninstalls the app or clears its data | `ProjectStoreModule.kt` (native module `ProjectStore`), wrapped by `src/lib/project-store.ts` |
| Copies of picked photos | `cacheDir/media/<uuid>/<name>` | Deleted right after use; Android may clear the cache at any time | `MediaFileModule.kt` (native module `MediaFile`), wrapped by `src/lib/media-file.ts` |
| AI models | `filesDir/abra-models/<role>-<12 hash characters>.onnx` plus a `.yml` beside it (Android); `Library/Caches/abra-models` on iOS | The user deletes the model. A new version of a model replaces the old files | `packages/react-native/alakazam/src/models.rs` (Rust) |
| The edit session (images, masks, preloads, sliders, undo steps) | Memory only | The editor closes. `saveOpenProject` saves what needs keeping first | `src/state/*`, `src/lib/editor-session.ts` |

### A project folder

`filesDir/projects/<id>/` holds one project. The id is a UUID made on the native side, and a name with a path separator in it is refused.

```text
projects/<id>/
├── project.json          # name, timestamps, the edit steps (see below)
├── source.<ext>          # the untouched original, copied in once and never rewritten
├── result-<time>.webp    # the final image at the cursor (or .jpg when WebP cannot encode it)
├── skin-mask.webp        # the skin mask the AI model made for the original, as lossless gray
├── person-<n>.webp       # a mask for each person found in the photo (older projects: people/person-<n>.webp)
├── makeup-<n>.webp       # the masks the face models made for the original (eyes, brows, lips, skin, nose of each face)
└── checkpoint-<n>.webp   # the pixels after a blemish or red-eye step
```

- **Making one:** `createProject` copies the photo from its `content://` or `file://` URI into the folder and writes a `project.json` with an empty timeline. The photo in the camera roll is never touched.
- **`project.json`** is the `Project` type in `src/lib/project-store.ts`. It holds `version`, `id`, `name`, `sourceFile`, `resultFile`, `skinMaskFile`, `makeupMasks` (each mask's name and file), `people` (their boxes), `timeline`, `cursor`, `createdAt` and `modifiedAt`. `dir` (the folder as a `file://` URI) is filled in when the project is read and is never saved.
- **The timeline** is every step in the order it was done, including undone ones that can be redone. Each step holds the slider values and applied actions after it, so an edit is a recipe and not pixels. `cursor` is how many steps are applied.
- **Pixels are saved only where a recipe cannot rebuild them:** a blemish or red-eye step bakes its result into a `checkpoint-<n>.webp`, so reopening never runs the AI tool again. Everything else, makeup layers included, is replayed from the recipe on top of the original or the last checkpoint.
- **The result image** is saved so the grid and the next open can show the picture before anything is replayed. Each save uses a new name (`result-<time>`) so the image cache never shows an old one, and `writeProject` deletes the earlier `result*` files.
- **Saving a project is safe against a crash.** `project.json` is written to `project.json.tmp` and then renamed over the old file, so a crash leaves the old file, never half of a new one.
- **When it saves:** `saveOpenProject` in `src/lib/editor-session.ts` runs when the editor closes and when the app leaves the foreground, because Android can kill a background app without warning. It does nothing if nothing changed, or while the project is still loading.
- **Opening:** `restoreProjects` lists the folders at startup. A folder with no readable `project.json` is skipped. `openEditorSession` shows `result-*` at once, then reads the source and rebuilds the base from the saved checkpoints.

### The in-memory list

`src/state/projects.ts` keeps `projects`, a `BehaviorSubject<Project[]>`, newest first. `addProject`, `updateProject` and `removeProjects` change the list first and then call the native module, so the grid updates at once.

### Cached copies of photos

The Camera Roll gives `content://` URIs, which the Rust library cannot open by path. `copyMediaToCache` copies one to `cacheDir/media/<uuid>/` and returns a `file://` URI. `deleteMediaFromCache` removes it and its folder, and only touches files inside the cache.

### AI rules

AI is for the moment it is used, and it runs once. Everything below follows from those two rules.

- **Never at startup.** Opening a photo or a project runs no model. A model runs when the user uses a control that needs it (a skin, person or makeup control), and not before. Do not start one in the background "to be ready".
- **Once per photo.** What a model finds in the original does not change, so it is found once, kept in memory for the session, and saved. An edit that is replayed, a thumbnail, a second control, another section and a reopen all reuse it. Do not run a model again because the base changed or a size is different: resize what was found.
- **Save everything needed to rebuild.** Whatever a model made that the edits need is saved to the project's storage as soon as it exists: masks as lossless gray WebP files, and data (boxes, points, colors, confidences, names) in `project.json`. Together they must be enough to open the project and replay every edit with **no model installed and none run**. A project must open on a device that never downloaded the models. Today that is the skin mask (`skinMaskFile`), the people (`people` and their masks) and the face masks (`makeupMasks`); a new model's results go the same way.
- **Save when first made, load on open.** `saveOpenProject` writes them the first time they exist (like `skinMaskFile`) and never again; `loadAnalysis` in `editor-session.ts` reads them back before anything is replayed and hands them to the code that would have run the model (for a preload, `provideAnalysis`). Check that an edit's replay path cannot reach a model call when the saved result is there.
- **A step that bakes pixels** (blemish, red-eye) saves them as a `checkpoint-<n>.webp`, for the same reason.
- **The only exception** is a project saved before its results were kept: the model runs once the first time its results are needed, and they are saved, so the next open needs none.

### AI models

Models are not in a project and are shared by all of them. `models.rs` reads a catalog from S3, and downloads a model to a `.part` file, checks its SHA-256 and only then renames it to its final name, so a file with a final name is always valid. The library only ever gets a path (`aiModelPath(role)`); downloading and locations belong to the app. Whether the user declined the download offer is held in memory (`aiModelsOfferDeclined`) and resets on restart.

### Not stored

There is no saved settings file. The theme starts from the system's and a choice made in the app is not kept (`src/lib/theme.tsx` says to add `@react-native-async-storage/async-storage` for that), and the editor's selected section, focused control and person selection reset when the editor opens. Exporting or saving a finished image to the camera roll is not done yet; `CameraRoll` is only used to read photos.
