const path = require('path');
const { getDefaultConfig, mergeConfig } = require('@react-native/metro-config');

const projectRoot = __dirname;
const workspaceRoot = path.resolve(projectRoot, '../..');
const defaultConfig = getDefaultConfig(projectRoot);

// In this npm workspace, packages hoisted to the repo root would otherwise resolve these from the root,
// where other workspaces may pull in other versions. Two Reacts or React Natives in one bundle fail at startup.
const SINGLETONS = ['react', 'react-native'];
const appOrigin = path.join(projectRoot, 'package.json');

const existingBlockList = defaultConfig.resolver.blockList ?? [];

/** @type {import('@react-native/metro-config').MetroConfig} */
const config = {
  // Hoisted dependencies and the workspace's React Native bindings live outside this folder. The workspace
  // root itself isn't watched: it holds the Rust target/ directory and other apps.
  watchFolders: [path.join(workspaceRoot, 'node_modules'), path.join(workspaceRoot, 'packages/react-native')],
  resolver: {
    nodeModulesPaths: [path.join(projectRoot, 'node_modules'), path.join(workspaceRoot, 'node_modules')],
    resolveRequest: (context, moduleName, platform) => {
      // `@/src/...` aliases the app root, mirroring the `paths` entry in tsconfig.json.
      if (moduleName.startsWith('@/')) {
        return context.resolveRequest(context, path.join(projectRoot, moduleName.slice(2)), platform);
      }
      const isSingleton = SINGLETONS.some((name) => moduleName === name || moduleName.startsWith(`${name}/`));
      const origin = isSingleton ? { ...context, originModulePath: appOrigin } : context;
      return context.resolveRequest(origin, moduleName, platform);
    },
    // Native build output churns during Gradle builds and crashes Metro's file watcher.
    blockList: [
      ...(Array.isArray(existingBlockList) ? existingBlockList : [existingBlockList]),
      /[\\/]android[\\/](app[\\/])?(build|\.cxx|\.gradle)[\\/].*/,
    ],
  },
};

module.exports = mergeConfig(defaultConfig, config);
