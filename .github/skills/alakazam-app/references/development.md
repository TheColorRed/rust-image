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

### Looking at a project's files on the phone

Projects live in the app's private storage (`files/projects/<id>/`, see the [mobile structure document](./mobile-structure.md#a-project-folder)). `adb` reads them with `run-as`, which only works for the debug build, `com.alakazam.mobile.debug` (the release package is not debuggable). Put the whole remote command in quotes, and give paths relative to the app's data folder: `files/projects/...`, not `./projects/...`.

```bash
# Every project, newest first (the folder name is the project id)
adb shell "run-as com.alakazam.mobile.debug ls -lt files/projects"

# Every file of one project
adb shell "run-as com.alakazam.mobile.debug find files/projects/<id> -type f"

# The same with sizes, or only the masks
adb shell "run-as com.alakazam.mobile.debug ls -lR files/projects/<id>"
adb shell "run-as com.alakazam.mobile.debug find files/projects/<id> -name '*.webp'"

# Every file of every project
adb shell "run-as com.alakazam.mobile.debug find files/projects -type f"

# Copy a file to this computer (project.json shows the steps and what was saved)
adb exec-out run-as com.alakazam.mobile.debug cat files/projects/<id>/project.json > project.json
```

To find which project is the photo being tested, take the newest folder, or search for a control's key. `run-as` does not expand `*` itself, so run the search through `sh -c`: `adb shell "run-as com.alakazam.mobile.debug sh -c 'grep -l slider-blush files/projects/*/project.json'"`.
