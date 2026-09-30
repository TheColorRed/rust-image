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
---

# Alakazam App

The app is split into two different apps, a desktop app located at `/apps/alakazam` and a mobile app located at `/apps/mobile`.

- The desktop app is an Electron-based desktop application that uses React for the frontend. The app provides a user interface for image manipulation using the Abra library as its backend.
- The mobile app is located at `/apps/mobile` and uses React Native for the frontend. It provides a user interface for image manipulation using the Abra library as its backend.

## Structure

### Desktop

See the [structure document](./references/desktop-structure.md) for a detailed breakdown of the directories and files that make up the Alakazam desktop app.

### Mobile

See the [structure document](./references/mobile-structure.md) for a detailed breakdown of the directories and files that make up the Alakazam mobile app.

## Bindings

The Alakazam App integrates with the Abra library using both Node and uniffi bindings. The Node bindings are used for the desktop app, while the uniffi bindings are used for the mobile app.

### Node

See the [bindings document](./references/node-bindings.md) for information on how the Alakazam App integrates with the Abra library using Node bindings generated with `napi-rs` for usage in the desktop app.

### Uniffi

See the [uniffi bindings document](./references/uniffi-bindings.md) for information on how the Alakazam App integrates with the Abra library using uniffi for usage in the mobile app.

## Development

See the [development document](./references/development.md) for instructions on how to set up a development environment for the Alakazam App, including building and running the application.
