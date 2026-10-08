# Alakazam desktop

## Windows download

The **Build downloadable apps** GitHub Actions workflow creates a portable
Windows x64 executable, `Alakazam-windows-x64.exe`.

1. Open **Actions > Build downloadable apps > Run workflow**.
2. Enable **Build the Windows desktop app**. The mobile builds can be disabled.
3. After the Windows job succeeds, download the `.exe` from the run's artifacts.

The executable includes Electron, the production JavaScript bundles, and the
native Rust addon. It extracts its application files on launch; it does not need
a separate Node.js or Rust installation and is not an installer.

The workflow uses Electron Packager to assemble the application directory and
Electron Builder's `portable` target to wrap it in a single executable.
The upload step uses `actions/upload-artifact@v7` with `archive: false`, so the
download is the executable rather than a ZIP containing it.

The executable is currently unsigned. Windows may display an unknown-publisher
or SmartScreen warning. Signing requires a Windows code-signing certificate and
separate CI signing configuration.

## Production build

From the repository root, build the native binding and application:

```powershell
npm run build-bindings
node apps\alakazam\scripts\move-bindings.mjs
$env:NODE_ENV = 'production'
npm run build -w apps/alakazam/desktop
```

The app build type-checks without emitting TypeScript files, then produces the
webpack bundles. This keeps TypeScript from overwriting the bundled Electron
entry point. CI stages the renderer as `dist/client`, packages the app, and
uploads the portable executable.

The Rust build compiles TurboJPEG from source using CMake and Visual Studio.
Keep the checked-in `Cargo.lock`: its `cmake` dependency supports Visual Studio
2026, used by the current `windows-latest` runner. Older `cmake` releases can
panic when detecting that Visual Studio version.
