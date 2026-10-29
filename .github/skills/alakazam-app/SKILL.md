---
name: alakazam-app
description: The Alakazam App is a Electron-based desktop application that uses React for the frontend. It provides a user interface for image manipulation using the Abra library as its backend. This app is written in TypeScript/JavaScript for both the frontend and backend.
metadata:
  keywords:
    - alakazam
    - electron
    - react
    - abra
    - image-manipulation
    - desktop-app
    - typescript
    - javascript
    - plugin
---

# Alakazam App

The app is split into two different apps, a desktop app located at `/apps/alakazam/desktop` and a mobile app located at `/apps/alakazam/mobile`.

- The desktop app is an Electron-based desktop application that uses React for the frontend. The app provides a user interface for image manipulation using the Abra library as its backend.
- The mobile app is located at `/apps/alakazam/mobile` and uses React Native for the frontend. It provides a user interface for image manipulation using the Abra library as its backend.

## Structure

### Desktop

See the [structure document](./references/desktop-structure.md) for a detailed breakdown of the directories and files that make up the Alakazam desktop app.

### Mobile

See the [structure document](./references/mobile-structure.md) for a detailed breakdown of the directories and files that make up the Alakazam mobile app.

## Usability

The app is for people who are not artists. See the [usability document](./references/usability.md) for how controls are named
(plain words, not trade words), how common choices are offered as named swatches, what a control should show when it opens, and
how a control that applies to a person behaves with one person or several. Read it before adding or renaming an edit control.

## Image work stays off the JavaScript thread

**The app's JavaScript never does the image work.** It describes what to make and changes settings; Rust (the Abra library and the Alakazam native crate) decodes, resizes, composes, filters and draws, on its own threads. A native call blocks the JavaScript thread until it returns, so every pixel operation written in TypeScript freezes the screen for as long as it takes: sliders stop following the finger, and a style switch or a pick looks like a hang. This is the same rule as "the library does the work", applied to threads, and it covers plugins as much as the editor.

- **Do not run pixel work in a loop or in an event handler.** `Image.read`, `resize`, `rotate`, `asImage`, `apply(...)`, `draw...` and the like do not belong in a slider's `onChange`, a redraw, a render, or a per-cell loop that runs when a person changes a control. A loop that does image work in TypeScript is a missing library primitive: add it to the library first (`abra-library` skill) and call it once.
- **Show a canvas, do not flatten it.** Give the preview the `Canvas` (`ImagePreview.fromCanvas`) and let the engine compose it when it has changed, on its own thread. Do not call `asImage()` and copy the result into a preview.
- **A drag changes properties, it does not rebuild.** Build the canvas, layers and effects once for a set of inputs. A slider then calls cheap setters on what exists (`setRotation`, `setScale`, `set_live_effects`), which only mark it changed, and the engine draws it. Rebuild only for a structural change (other pictures, another style, a new size), and never once per move of a drag.
- **Work that is slow in itself goes to a worker.** A decode, a large resize or a model run is made an async native call (as `runAiModelAsync` is) or is done by the library in the background, with the control showing progress, so the editor keeps drawing and answering touches. Cache the result for the inputs that made it, so a change that does not touch those inputs (another layout of the same photos) does not make it again.
- **Check before adding code that touches pixels.** If it runs when a control changes, ask what thread it runs on. If the answer is the JavaScript thread, it is in the wrong place.

## AI

AI models run only when a control that needs one is used, and run once per photo. Whatever they find that an edit needs is saved to the project (masks as files, data in `project.json`) so a project opens and replays with no model installed or run. See the AI rules in the [mobile structure document](./references/mobile-structure.md#ai-rules).

## Plugins

The Alakazam App supports plugins to extend its functionality. Plugins can be used to add new image manipulation features, integrate with external services, or customize the user interface. See the [plugins document](./references/plugins.md) for information on how to create and use plugins within the Alakazam App.

## Bindings

The Alakazam App integrates with the Abra library using both Node and uniffi bindings. The Node bindings are used for the desktop app, while the uniffi bindings are used for the mobile app.

### Node

See the [bindings document](./references/node-bindings.md) for information on how the Alakazam App integrates with the Abra library using Node bindings generated with `napi-rs` for usage in the desktop app.

### Uniffi

See the [uniffi bindings document](./references/uniffi-bindings.md) for information on how the Alakazam App integrates with the Abra library using uniffi for usage in the mobile app.

Both apps follow the `abra-library` skill's [applications document](../abra-library/references/applications.md): the library does the work and the app is a thin caller.

## Development

See the [development document](./references/development.md) for instructions on how to set up a development environment for the Alakazam App, including building and running the application.
