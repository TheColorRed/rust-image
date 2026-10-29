#!/usr/bin/env node
// Watches plugin folders and uploads built files to an Alakazam app development server.
import Enquirer from 'enquirer';
import chalk from 'chalk';
import chokidar from 'chokidar';
import fg from 'fast-glob';
import { createReadStream, existsSync, readFileSync, writeFileSync } from 'node:fs';
import { stat } from 'node:fs/promises';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';
import { buildPlugins, warmPluginBuilder } from './plugin-builder.mjs';

const { Input } = Enquirer;
const args = process.argv.slice(2);
if (args[0] === 'dev' || args[0] === 'dev-plugin') args.shift();

const options = new Map();
const searchRoots = [];
for (let index = 0; index < args.length; index++) {
  const value = args[index];
  if (['--url', '--root'].includes(value)) {
    const optionValue = args[++index];
    if (!optionValue || optionValue.startsWith('--')) throw new Error(`${value} requires a value`);
    options.set(value, optionValue);
  } else if (value.startsWith('--')) {
    throw new Error(`Unknown option: ${value}`);
  } else {
    searchRoots.push(value);
  }
}

function workspaceRootFrom(start) {
  let current = path.resolve(start);
  while (true) {
    if (existsSync(path.join(current, 'packages', 'plugins', 'plugin-toolkit', 'package.json')) &&
        existsSync(path.join(current, 'apps', 'alakazam', 'mobile'))) return current;
    const parent = path.dirname(current);
    if (parent === current) return undefined;
    current = parent;
  }
}
const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const scriptWorkspace = path.resolve(scriptDir, '..', '..', '..');
const inferredRoot = workspaceRootFrom(scriptWorkspace) ?? workspaceRootFrom(process.cwd()) ?? process.cwd();
const roots = options.has('--root')
  ? [path.resolve(options.get('--root'))]
  : searchRoots.length > 0
    ? searchRoots.map(value => path.resolve(value))
    : [inferredRoot];

function findManifestFiles() {
  return roots.flatMap(root => fg.sync('**/plugin.yml', {
    cwd: root,
    onlyFiles: true,
    ignore: ['**/node_modules/**', '**/dist/**', '**/target/**'],
  }).map(file => path.resolve(root, file)));
}

const invalidManifestWarnings = new Map();

function skipInvalidManifest(manifestFile, reason) {
  if (invalidManifestWarnings.get(manifestFile) !== reason) {
    console.warn(`Ignoring ${manifestFile}: ${reason}. Waiting for a valid plugin.yml.`);
    invalidManifestWarnings.set(manifestFile, reason);
  }
  return [];
}

function discoverPlugins(manifests = findManifestFiles()) {
  return manifests.flatMap(manifestFile => {
    const dir = path.dirname(manifestFile);
    let manifestText;
    let manifest;
    try {
      manifestText = readFileSync(manifestFile, 'utf8');
      manifest = parse(manifestText);
    } catch (error) {
      return skipInvalidManifest(manifestFile, `could not parse YAML (${error.message})`);
    }
    if (!manifest || typeof manifest !== 'object' || Array.isArray(manifest)) {
      return skipInvalidManifest(manifestFile, 'the document must contain a plugin object');
    }
    if (typeof manifest?.id !== 'string' || !/^[a-z0-9]+(?:-[a-z0-9]+)*\.[a-z0-9]+(?:-[a-z0-9]+)*$/.test(manifest.id)) {
      return skipInvalidManifest(manifestFile, 'the plugin id is missing or invalid');
    }
    if (typeof manifest.name !== 'string' || manifest.name.trim() === '' ||
        !(['string', 'number'].includes(typeof manifest.version)) || String(manifest.version).trim() === '') {
      return skipInvalidManifest(manifestFile, 'name and version are required');
    }
    const contributes = manifest.contributes ?? {};
    if (manifest.contributes !== undefined && (!contributes || typeof contributes !== 'object' || Array.isArray(contributes))) {
      return skipInvalidManifest(manifestFile, 'contributes must be an object');
    }
    const controls = contributes.controls ?? [];
    const scripts = contributes.scripts ?? [];
    const workspaces = contributes.workspaces ?? [];
    if ((contributes.controls !== undefined && !Array.isArray(contributes.controls)) ||
        (contributes.scripts !== undefined && !Array.isArray(contributes.scripts)) ||
        (contributes.workspaces !== undefined && !Array.isArray(contributes.workspaces)) ||
        controls.some(group => !group || typeof group !== 'object' || typeof group.root !== 'string' || group.root.trim() === '' ||
          !Array.isArray(group.items) || group.items.length === 0 || group.items.some(item =>
            !item || typeof item !== 'object' || typeof item.path !== 'string' || item.path.trim() === '' ||
            typeof item.entry !== 'string' || !item.entry.endsWith('.js'))) ||
        scripts.some(script => !script || typeof script !== 'object' ||
          typeof script.id !== 'string' || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(script.id) ||
          typeof script.title !== 'string' || script.title.trim() === '' ||
          !Array.isArray(script.capabilities) || script.capabilities.length === 0 ||
          typeof script.entry !== 'string' || !script.entry.endsWith('.js')) ||
        workspaces.some(workspace => !workspace || typeof workspace !== 'object' ||
          typeof workspace.id !== 'string' || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(workspace.id) ||
          typeof workspace.title !== 'string' || workspace.title.trim() === '' ||
          typeof workspace.entry !== 'string' || !workspace.entry.endsWith('.js'))) {
      return skipInvalidManifest(manifestFile, 'controls, scripts and workspaces must list JavaScript entries');
    }
    invalidManifestWarnings.delete(manifestFile);
    const hasEntries = controls.some(group => group.items.length > 0) || scripts.length > 0 || workspaces.length > 0;
    return hasEntries
      ? { dir, manifestPath: manifestFile, manifest, lastBuiltManifestText: manifestText, building: false, queued: false }
      : [];
  });
}

const initialManifestFiles = findManifestFiles();
let plugins = discoverPlugins(initialManifestFiles);
if (plugins.length === 0 && initialManifestFiles.length === 0) throw new Error(`No plugin.yml found under ${roots.join(', ')}`);

function refreshPlugins() {
  const previous = new Map(plugins.map(plugin => [plugin.dir, plugin]));
  plugins = discoverPlugins().map(found => {
    const current = previous.get(found.dir);
    if (!current) return found;
    current.manifest = found.manifest;
    current.manifestPath = found.manifestPath;
    return current;
  });
  return plugins;
}

async function connectToApp() {
  let enteredUrl = options.get('--url');
  if (!enteredUrl) {
    const historyPath = path.join(os.homedir(), '.alakazam-plugin-address-history.json');
    const historyStore = {
      get(key) {
        try { return JSON.parse(readFileSync(historyPath, 'utf8'))[key]; }
        catch { return undefined; }
      },
      set(key, value) {
        if (key === 'values') writeFileSync(historyPath, JSON.stringify({ values: value }), 'utf8');
      },
    };
    const prompt = new Input({
      name: 'url',
      message: 'Enter a server address shown on the app Plugins page',
      history: { store: historyStore, autosave: true },
    });
    // Enquirer maps history to Alt+Up by default; use plain arrows for this single-line address field too.
    prompt.up = prompt.altUp.bind(prompt);
    prompt.down = prompt.altDown.bind(prompt);
    enteredUrl = await prompt.run();
  }
  const address = enteredUrl.trim();
  const serverUrl = new URL(address.includes('://') ? address : `http://${address}`);
  if (serverUrl.protocol !== 'http:' || serverUrl.pathname !== '/' || serverUrl.search || serverUrl.hash) {
    throw new Error('Enter the app server address exactly as shown (for example, 192.168.1.20:43821)');
  }
  serverUrl.pathname = '/';
  await waitForApp(serverUrl);
  console.log(chalk.green(`Connected to Alakazam at ${serverUrl.host}.`));
  return serverUrl;
}

const reconnectDelayMs = 1_000;
const healthCheckIntervalMs = 2_000;
let reconnecting = false;
let reconnectPromise;

async function waitForApp(serverUrl) {
  if (reconnectPromise) return reconnectPromise;
  reconnectPromise = reconnectToApp(serverUrl).finally(() => { reconnectPromise = undefined; });
  return reconnectPromise;
}

async function reconnectToApp(serverUrl) {
  if (reconnecting) {
    await new Promise(resolve => setTimeout(resolve, reconnectDelayMs));
  }
  while (true) {
    try {
      await requestOnce(serverUrl, 'GET', '/health');
      if (reconnecting) console.log(chalk.green(`Reconnected to Alakazam at ${serverUrl.host}.`));
      reconnecting = false;
      return;
    } catch {
      if (!reconnecting) console.warn(chalk.yellow(`Lost connection to Alakazam at ${serverUrl.host}; waiting for it to come back...`));
      reconnecting = true;
      await new Promise(resolve => setTimeout(resolve, reconnectDelayMs));
    }
  }
}

async function appRequest(serverUrl, method, route, bodyPath, extraHeaders = {}) {
  while (true) {
    await waitForApp(serverUrl);
    try {
      return await requestOnce(serverUrl, method, route, bodyPath, extraHeaders);
    } catch (error) {
      console.warn(chalk.yellow(`Connection to Alakazam was interrupted; retrying ${method} ${route} when it is available.`));
      await new Promise(resolve => setTimeout(resolve, reconnectDelayMs));
    }
  }
}

function monitorApp(serverUrl) {
  const timer = setInterval(async () => {
    if (reconnecting) return;
    try {
      await requestOnce(serverUrl, 'GET', '/health');
    } catch {
      void waitForApp(serverUrl);
    }
  }, healthCheckIntervalMs);
  timer.unref();
}

async function requestOnce(serverUrl, method, route, bodyPath, extraHeaders = {}) {
  const size = bodyPath ? (await stat(bodyPath)).size : 0;
  return new Promise((resolve, reject) => {
    const request = http.request(new URL(route, serverUrl), {
      method,
      headers: {
        ...extraHeaders,
        ...(bodyPath ? { 'Content-Type': 'application/octet-stream', 'Content-Length': size } : {}),
      },
      timeout: 30_000,
    }, response => {
      let body = '';
      response.setEncoding('utf8').on('data', chunk => { body += chunk; });
      response.on('end', () => {
        if (response.statusCode && response.statusCode >= 200 && response.statusCode < 300) resolve(body);
        else reject(new Error(`App server returned ${response.statusCode}: ${body || 'request failed'}`));
      });
    });
    request.once('error', reject);
    request.once('timeout', () => request.destroy(new Error('Timed out connecting to the app server')));
    if (bodyPath) {
      const input = createReadStream(bodyPath);
      input.once('error', error => { request.destroy(error); reject(error); });
      input.pipe(request);
    } else {
      request.end();
    }
  });
}

const serverUrl = await connectToApp();
monitorApp(serverUrl);
console.log('Preparing plugin builder...');
await warmPluginBuilder();
console.log('Plugin builder ready.');
function queuePluginChange(plugin) {
  if (plugin.removed) return;
  if (!plugin.changeLogged) {
    console.log(chalk.cyan(`Change detected for ${plugin.manifest.id}; building and syncing...`));
    plugin.changeLogged = true;
  }
  clearTimeout(plugin.debounceTimer);
  plugin.debounceTimer = setTimeout(() => {
    plugin.debounceTimer = undefined;
    plugin.changeLogged = false;
    void buildAndPush(plugin);
  }, 250);
}

async function buildAndPush(plugin) {
  if (plugin.removed) return;
  if (plugin.building) {
    if (!plugin.queued) console.log(chalk.yellow(`Another change for ${plugin.manifest.id} was queued during the build.`));
    plugin.queued = true;
    return;
  }
  plugin.building = true;
  try {
    await buildPlugins([plugin.dir], { devMode: true });
    if (plugin.removed) return;
    plugin.manifest = parse(readFileSync(plugin.manifestPath, 'utf8'));
    plugin.lastBuiltManifestText = readFileSync(plugin.manifestPath, 'utf8');
    watchPlugin(plugin);
    const pluginHeader = { 'X-Plugin-Id': plugin.manifest.id };
    await appRequest(serverUrl, 'POST', '/begin', undefined, pluginHeader);
    const sendFile = async (relativePath, localPath) => appRequest(
      serverUrl,
      'POST',
      '/file',
      localPath,
      { ...pluginHeader, 'X-Plugin-Path': Buffer.from(relativePath, 'utf8').toString('base64url') },
    );
    await sendFile('plugin.yml', plugin.manifestPath);
    const builtEntries = new Set([
      ...(plugin.manifest.contributes?.controls ?? []).flatMap(group => (group.items ?? []).map(item => item.entry)),
      ...(plugin.manifest.contributes?.scripts ?? []).map(script => script.entry),
      ...(plugin.manifest.contributes?.workspaces ?? []).map(workspace => workspace.entry),
    ]);
    for (const file of plugin.manifest.files ?? []) {
      const source = builtEntries.has(file.path) ? path.join(plugin.dir, 'dist', ...file.path.split('/')) : path.join(plugin.dir, ...file.path.split('/'));
      await sendFile(file.path, source);
    }
    if (plugin.removed) return;
    await appRequest(serverUrl, 'POST', '/complete', undefined, pluginHeader);
    console.log(chalk.green(`Sent ${plugin.manifest.id} to Alakazam.`));
  } catch (error) {
    console.error(error.message);
  } finally {
    plugin.building = false;
    if (plugin.queued && !plugin.removed) { plugin.queued = false; void buildAndPush(plugin); }
  }
}

console.log(`Searching for plugin.yml under ${roots.join(', ')}.`);
console.log(`Watching ${plugins.length} plugin folder(s). Save a plugin file to build and sync it.`);
const pluginWatchers = new Map();
const parentWatchers = new Map();

function pluginChangeIsRelevant(plugin, relativePath) {
  if (relativePath === '' || relativePath === '.') return true;
  const parts = relativePath.split('/');
  if (parts.some(part => part === 'dist' || part === 'node_modules' || part === 'target' || part === '.git')) return false;
  if (relativePath === 'plugin.yml' || relativePath === 'package.json' || relativePath === 'package-lock.json' ||
      relativePath === 'src' || relativePath.startsWith('src/')) return true;
  const builtEntries = new Set([
    ...(plugin.manifest.contributes?.controls ?? []).flatMap(group => (group.items ?? []).map(item => item.entry)),
    ...(plugin.manifest.contributes?.scripts ?? []).map(script => script.entry),
    ...(plugin.manifest.contributes?.workspaces ?? []).map(workspace => workspace.entry),
  ]);
  return (plugin.manifest.files ?? []).some(file => !builtEntries.has(file.path) && file.path === relativePath);
}

function handlePluginChange(plugin, changedPath) {
  const relativePath = path.relative(plugin.dir, changedPath).replaceAll('\\', '/');
  if (!pluginChangeIsRelevant(plugin, relativePath)) return;
  if (relativePath === 'plugin.yml') {
    let manifestText = '';
    try { manifestText = readFileSync(plugin.manifestPath, 'utf8'); } catch {}
    if (manifestText === plugin.lastBuiltManifestText) return;
    if (plugin.building) {
      clearTimeout(plugin.manifestEventTimer);
      plugin.manifestEventTimer = setTimeout(() => {
        plugin.manifestEventTimer = undefined;
        let latest = '';
        try { latest = readFileSync(plugin.manifestPath, 'utf8'); } catch {}
        if (latest !== plugin.lastBuiltManifestText) queuePluginChange(plugin);
      }, 400);
      return;
    }
  }
  queuePluginChange(plugin);
}

function watchPlugin(plugin) {
  const existing = pluginWatchers.get(plugin.dir);
  if (existing) return;
  const watcher = chokidar.watch(plugin.dir, {
    ignoreInitial: true,
    atomic: true,
    ignored: /(^|[\\/])(?:node_modules|dist|target|\.git)(?:[\\/]|$)/,
  });
  watcher.on('all', (_event, changedPath) => handlePluginChange(plugin, path.resolve(changedPath)));
  watcher.on('error', error => console.warn(`Plugin watcher error for ${plugin.dir}: ${error.message}`));
  pluginWatchers.set(plugin.dir, watcher);
}

function watchPluginParent(plugin) {
  const parent = path.dirname(plugin.dir);
  if (parentWatchers.has(parent)) return;
  let timer;
  try {
    const watcher = chokidar.watch(parent, {
      depth: 1,
      ignoreInitial: true,
      atomic: true,
      ignored: /(^|[\\/])(?:node_modules|dist|target|\.git)(?:[\\/]|$)/,
    });
    watcher.on('all', (event, changedPath) => {
      if (event !== 'addDir' && event !== 'unlinkDir' && path.basename(changedPath) !== 'plugin.yml') return;
      clearTimeout(timer);
      timer = setTimeout(() => {
        const previousPlugins = new Map(plugins.map(each => [each.dir, each]));
        const knownDirs = new Set(previousPlugins.keys());
        const discovered = refreshPlugins();
        const discoveredDirs = new Set(discovered.map(each => each.dir));
        for (const [dir, pluginWatcher] of pluginWatchers) {
          if (!discoveredDirs.has(dir)) {
            const stopped = previousPlugins.get(dir);
            if (stopped) {
              stopped.removed = true;
              clearTimeout(stopped.debounceTimer);
              clearTimeout(stopped.manifestEventTimer);
              stopped.changeLogged = false;
              if (!existsSync(dir) || !existsSync(stopped.manifestPath)) {
                console.log(`Plugin ${stopped.manifest.id} is no longer being watched: its plugin.yml or folder was deleted.`);
              }
            }
            pluginWatcher.close();
            pluginWatchers.delete(dir);
          }
        }
        for (const found of discovered) {
          watchPlugin(found);
          if (!knownDirs.has(found.dir)) {
            console.log(`New plugin found for watching: ${found.manifest.id} (${found.dir})`);
            queuePluginChange(found);
            watchPluginParent(found);
          }
        }
      }, 150);
    });
    watcher.on('error', error => console.warn(`Plugin directory watcher error for ${parent}: ${error.message}`));
    parentWatchers.set(parent, watcher);
  } catch (error) {
    console.warn(`Could not watch plugin directory ${parent}: ${error.message}`);
  }
}

for (const plugin of plugins) {
  watchPlugin(plugin);
  watchPluginParent(plugin);
  console.log(`${plugin.manifest.id}: watching ${plugin.dir}`);
}
for (const manifestFile of initialManifestFiles) {
  const dir = path.dirname(manifestFile);
  if (!plugins.some(plugin => plugin.dir === dir)) watchPluginParent({ dir });
}
