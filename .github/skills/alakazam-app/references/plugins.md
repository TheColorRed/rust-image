## Alakazam Plugins

Alakazam plugins extend the functionality of the Alakazam platform. They allow developers to add new features, integrate with external services, and customize the behavior of the application. Each plugin typically consists of a set of components, configurations, and scripts that the Alakazam runtime can load and execute.

### Alakazam Team Plugins

Plugins maintained by the Alakazam team are located at `apps/alakazam/{desktop,mobile}/plugins/*`. These plugins are officially supported and regularly updated by the Alakazam team to ensure compatibility and stability within the platform.

### Migrating Plugins

When converting a plugin from a built-in feature to a plugin feature:

1. Make sure `@alakazam/plugin-toolkit` is generic and not tightly coupled to any specific plugin.
2. Move all the logic to the plugin needs into the plugin itself, ensuring that the plugin is self-contained and does not rely on built-in features. It should use the `@alakazam/plugin-toolkit` to interact with the application, or `@abra/core/*` to interact with the image processing core.
