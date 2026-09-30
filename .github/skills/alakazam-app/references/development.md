# Development

## Desktop App

When making sure changes run, don't start the app. It has already been started by the developer, and any changes to the code will automatically be picked up by the hot-reloading setup. This includes changes to the following areas:

- React components in the `/src/renderer` directory.
- Electron main process code in the `/src/server` directory.
- Bindings in the `/packages/node/alakazam` directory.

If the application tries to start a second time, it will crash the running instance which is a pain to have to restart every time. Instead, just make your code changes and see them reflected in the already running app.

## Mobile App

The mobile app uses React Native for the frontend and integrates with the Abra library through the `@alakazam/mobile` package in `/packages/react-native/alakazam`. It uses Metro as the bundler and supports hot-reloading for rapid development. Changes to the code will be automatically picked up without needing to restart the app. Somethings do require a manual restart such as a re-build of the rust bindings or changes to the native code.

The app is connected via `wireless debugging` so you should push the changes to the device to see them reflected in the running app if hot-reloading does not pick up the changes or this requires a manual restart/deployment of the app.
