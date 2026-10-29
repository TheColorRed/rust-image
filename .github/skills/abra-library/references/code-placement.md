# Sections

The library is organized into several sections that perform different types of image processing and effects. They each have functions that ....

## Adjustments

Adjustments modify the global color and tonal properties of an image, such as brightness, contrast, and saturation.

## Filters

Filters alters the spatial structure, texture, or appearance of an image, such as applying blur, sharpening, or artistic effects.

## Tools

Tools reuse adjustments and filters so that they don't create redundant computations, such as applying a series of adjustments or filters in a single operation like adjusting the brightness, contrast, and saturation together.

## Plugins

Plugins are added at runtime, not compiled in: a plugin is a folder with a `plugin.yml` (see `packages/react-native/alakazam/src/plugins`), and an app gains it by adding the folder, with no rebuild and no change to the app's code. It can add AI models (described entirely by their manifests), pipelines (lists of library operations) and TypeScript scripts, each with the capabilities it provides, and a feature asks for a capability and offers whatever plugins give it. Code a plugin needs in order to run belongs in the host (the AI runtime, the app's JavaScript engine), written once for every plugin, and never in the app for one plugin.

A Rust crate that an app links in, such as `collage`, is a library add-on and not a plugin. Do not implement a trait to make something a plugin; describe it in a `plugin.yml`.
