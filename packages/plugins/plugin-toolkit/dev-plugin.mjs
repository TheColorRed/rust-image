#!/usr/bin/env node
// Watches plugin folders and uploads built files to an Alakazam app development server.
import Enquirer from 'enquirer';
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

const { Input, Password } = Enquirer;
const args = process.argv.slice(2);
if (args[0] === 'dev' || args[0] === 'dev-plugin') args.shift();

const options = new Map();
const searchRoots = [];
for (let index = 0; index < args.length; index++) {
  const value = args[index];
  if (['--url', '--token', '--root'].includes(value)) {
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

function discoverPlugins() {
  const manifests = roots.flatMap(root => fg.sync('**/plugin.yml', {
    cwd: root,
    onlyFiles: true,
    ignore: ['**/node_modules/**', '**/dist/**', '**/target/**'],
  }).map(file => path.resolve(root, file)));
  return manifests.flatMap(manifestFile => {
    const dir = path.dirname(manifestFile);
    const manifestText = readFileSync(manifestFile, 'utf8');
    const manifest = parse(manifestText);
    if (typeof manifest?.id !== 'string' || !/^[a-z0-9]+(?:-[a-z0-9]+)*\.[a-z0-9]+(?:-[a-z0-9]+)*$/.test(manifest.id)) {
      throw new Error(`${manifestFile} has no valid plugin id`);
    }
    const hasEntries = (manifest.contributes?.controls ?? []).some(group => (group.items ?? []).length > 0) ||
      (manifest.contributes?.scripts ?? []).length > 0;
    return hasEntries
      ? { dir, manifestPath: manifestFile, manifest, lastBuiltManifestText: manifestText, building: false, queued: false }
      : [];
  });
}

let plugins = discoverPlugins();
if (plugins.length === 0) throw new Error(`No plugin.yml found under ${roots.join(', ')}`);

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
  const token = options.get('--token') ?? await new Password({
    name: 'token',
    message: 'Enter the access token shown on the app Plugins page',
  }).run();
  const address = enteredUrl.trim();
  const serverUrl = new URL(address.includes('://') ? address : `http://${address}`);
  if (serverUrl.protocol !== 'http:' || serverUrl.pathname !== '/' || serverUrl.search || serverUrl.hash) {
    throw new Error('Enter the app server address exactly as shown (for example, 192.168.1.20:43821)');
  }
  serverUrl.pathname = '/';
  await appRequest(serverUrl, token, 'GET', '/health');
  console.log(`Connected to Alakazam at ${serverUrl.host}.`);
  return { serverUrl, token };
}

async function appRequest(serverUrl, token, method, route, bodyPath, extraHeaders = {}) {
  const size = bodyPath ? (await stat(bodyPath)).size : 0;
  return new Promise((resolve, reject) => {
    const request = http.request(new URL(route, serverUrl), {
      method,
      headers: {
        Authorization: `Bearer ${token}`,
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

const { serverUrl, token } = await connectToApp();
console.log('Preparing plugin builder...');
await warmPluginBuilder();
console.log('Plugin builder ready.');
function queuePluginChange(plugin) {
  if (!plugin.changeLogged) {
    console.log(`Change detected for ${plugin.manifest.id}; building and syncing...`);
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
  if (plugin.building) {
    if (!plugin.queued) console.log(`Another change for ${plugin.manifest.id} was queued during the build.`);
    plugin.queued = true;
    return;
  }
  plugin.building = true;
  try {
    await buildPlugins([plugin.dir], { devMode: true });
    plugin.manifest = parse(readFileSync(plugin.manifestPath, 'utf8'));
    plugin.lastBuiltManifestText = readFileSync(plugin.manifestPath, 'utf8');
    watchPlugin(plugin);
    const pluginHeader = { 'X-Plugin-Id': plugin.manifest.id };
    await appRequest(serverUrl, token, 'POST', '/begin', undefined, pluginHeader);
    const sendFile = async (relativePath, localPath) => appRequest(
      serverUrl,
      token,
      'POST',
      '/file',
      localPath,
      { ...pluginHeader, 'X-Plugin-Path': Buffer.from(relativePath, 'utf8').toString('base64url') },
    );
    await sendFile('plugin.yml', plugin.manifestPath);
    const builtEntries = new Set([
      ...(plugin.manifest.contributes?.controls ?? []).flatMap(group => (group.items ?? []).map(item => item.entry)),
      ...(plugin.manifest.contributes?.scripts ?? []).map(script => script.entry),
    ]);
    for (const file of plugin.manifest.files ?? []) {
      const source = builtEntries.has(file.path) ? path.join(plugin.dir, 'dist', ...file.path.split('/')) : path.join(plugin.dir, ...file.path.split('/'));
      await sendFile(file.path, source);
    }
    await appRequest(serverUrl, token, 'POST', '/complete', undefined, pluginHeader);
    console.log(`Sent ${plugin.manifest.id} to Alakazam.`);
  } catch (error) {
    console.error(error.message);
  } finally {
    plugin.building = false;
    if (plugin.queued) { plugin.queued = false; void buildAndPush(plugin); }
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
        const knownDirs = new Set(plugins.map(each => each.dir));
        const discovered = refreshPlugins();
        const discoveredDirs = new Set(discovered.map(each => each.dir));
        for (const [dir, pluginWatcher] of pluginWatchers) {
          if (!discoveredDirs.has(dir)) {
            pluginWatcher.close();
            pluginWatchers.delete(dir);
          }
        }
        for (const found of discovered) {
          watchPlugin(found);
          if (!knownDirs.has(found.dir)) {
            console.log(`Found ${found.manifest.id}: ${found.dir}`);
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
