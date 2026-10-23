// What the plugin scripts need to agree on about a plugin's folder.
import { existsSync } from 'node:fs';
import path from 'node:path';
import ts from 'typescript';

/**
 * Where a plugin's built scripts are written: the `outDir` of its `tsconfig.json`, which may come from the config it extends, or the
 * plugin's own folder when it has none. The build writes there, and the packer and the tests look there for what the plugin.yml lists.
 * The plugin.yml still names the files as they are delivered (`lipstick.js`), whatever folder they are built in.
 */
export function outDirOf(pluginDir) {
  const configPath = path.join(pluginDir, 'tsconfig.json');
  if (!existsSync(configPath)) return pluginDir;
  const read = ts.readConfigFile(configPath, ts.sys.readFile);
  if (read.error) throw new Error(`${configPath} cannot be read: ${ts.flattenDiagnosticMessageText(read.error.messageText, '\n')}`);
  const { options } = ts.parseJsonConfigFileContent(read.config, ts.sys, pluginDir);
  return options.outDir ? path.resolve(options.outDir) : pluginDir;
}
