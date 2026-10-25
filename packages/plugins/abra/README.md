# @abra/core

Type declarations for the Abra API available to Alakazam plugins.

Install this package as a development dependency when authoring a plugin that
imports from `@abra/core`:

```sh
npm install --save-dev @abra/core
```

The package provides TypeScript declarations only. Alakazam provides the
runtime implementation when it loads the plugin. Supported declaration
subpaths include `@abra/core/adjustments`, `@abra/core/beauty`,
`@abra/core/blend`, `@abra/core/canvas`, `@abra/core/color`,
`@abra/core/drawing`, `@abra/core/fill`, `@abra/core/filters`,
`@abra/core/geometry`, `@abra/core/gradient`, `@abra/core/image`,
`@abra/core/mask`, `@abra/core/tools`, and `@abra/core/transform`.

Build the declarations from the repository root with:

```sh
npm run build --workspace=@abra/core
```

The build uses generated Abra bindings from the Alakazam repository.
