This folder contains the JavaScript bindings for the project. The `ffi-napi` package depends on native compilation (node-gyp), which requires the MSVC toolchain and a Windows CPython on Windows.

To ease installation on Windows, a `preinstall` script detects and sets a local `.npmrc` with the Windows Python path, avoiding MSYS Python being picked up.

Windows setup
1. Open **Developer PowerShell for Visual Studio 2022** (this ensures `cl.exe` and MSBuild are on PATH).
2. Run the helper script to prepare the environment and install:
   ```powershell
   cd bindings/javascript
   npm run setup-windows
   ```

If `npm install` still fails with a `ffi-napi` build error, make sure:
- Visual Studio is installed with the `Desktop development with C++` workload.
- You have a Windows CPython on PATH (from python.org, not MSYS/Cygwin).
- Use a compatible Node LTS version if necessary, e.g. Node 18 via `nvm`.

This folder's `.npmrc` file is intentionally ignored by git and is written by the `preinstall` script when needed.
