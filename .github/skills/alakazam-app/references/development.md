# Development

When making sure changes run, don't start the app. It has already been started by the developer, and any changes to the code will automatically be picked up by the hot-reloading setup. This includes changes to the following areas:

- React components in the `/src/renderer` directory.
- Electron main process code in the `/src/server` directory.
- Bindings in the `/packages/node/alakazam` directory.

If the application tries to start a second time, it will crash the running instance which is a pain to have to restart every time. Instead, just make your code changes and see them reflected in the already running app.
