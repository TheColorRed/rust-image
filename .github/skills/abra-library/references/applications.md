# Applications and the Library

These rules apply to everything that uses the Abra library, not to one platform: Rust applications (the examples), the Electron app, the React Native app, bindings in any language, and any kind of application added later.

## The library does the work, applications are thin callers

An application connects to the library's API and tells it what to do. It does not do the work.

- **The library does the work.** If a feature needs computation (a loop, a formula, a search, a merge, a conversion), that computation is a library operation, whatever the feature is about. If the operation is missing, add it to the library as a generic, reusable function (see [reuse first](./reuse-first.md)) and make it available to applications. Do not write it in the application or tool.
- **Keep data on the library side.** Pass the library's own values from one call to the next. Do not copy large data out of the library to change it and copy it back. Read values out only to show them or to branch on a small result. Across a language boundary every crossing copies the whole thing.
- **Compose, don't compute.** A tool or feature in an application should read as a short list of library calls.
- **A loop in an application is a sign of a missing primitive.** Stop and propose the primitive first, then write the feature on top of it.
- **Library code stays free of application concerns.** Anything that needs the network, app storage, a UI or other platform services belongs in the application, which passes the library what it needs (a path, bytes, a value).

## Bindings mirror the library

A binding layer (for any language or platform) exposes what the library has and nothing more. It does not invent types, errors or helpers. A binding may adapt a signature only where the target language cannot carry the library's (for example, a tuple becomes an array), and says so where it does.
