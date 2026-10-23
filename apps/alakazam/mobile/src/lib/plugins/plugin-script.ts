/** The modules a plugin script can import, by name. Anything else a script imports is refused. */
export type ScriptModules = Readonly<Record<string, unknown>>;

/**
 * Runs a plugin script and returns what it exports. A script is a CommonJS module (its TypeScript is compiled to one when the plugin is
 * built), so it is given `module`, `exports` and a `require` that hands out only `modules`: the Abra bindings and what the app offers
 * plugins. A script can reach the app's globals all the same, and runs with all of the app's rights, so only plugins from a catalog that
 * checks their hash should be run.
 *
 * - `source`: The script's text.
 * - `modules`: What `require` can give, by the name the script imports it with.
 * - `name`: What to call the script in an error.
 */
export function runPluginScript(source: string, modules: ScriptModules, name: string): Record<string, unknown> {
  const module: { exports: Record<string, unknown> } = { exports: {} };
  const require = (id: string): unknown => {
    if (!Object.prototype.hasOwnProperty.call(modules, id)) {
      throw new Error(`The plugin script ${name} imports ${id}, which the app does not give to plugins.`);
    }
    return modules[id];
  };
  // eslint-disable-next-line no-new-func
  new Function('module', 'exports', 'require', source)(module, module.exports, require);
  return module.exports;
}
