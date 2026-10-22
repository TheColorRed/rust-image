/** A plugin the catalog (`catalog.yml`) can install: its id, and where its `plugin.yml` is and what it hashes to. */
export type CatalogPlugin = { id: string; uri: string; sha256: string };

/** What the device knows of a plugin that is installed. A plugin that cannot be used lacks some of the plugins it depends on. */
export type InstalledPlugin = { usable: boolean; missingDependencies: readonly string[] };

/**
 * The plugins to install, in order, so that the plugin `id` can be used: the plugins it depends on come before it, and what those depend
 * on before them. The catalog is the lookup: a dependency is found there by its id, and the plugin's own `plugin.yml` says which plugins
 * it needs (`dependenciesOf` reads that). A plugin that is installed and can be used is left as it is, and one that is installed but
 * cannot be used yet is not installed again: only what it is missing is. `id` itself is always in the plan when the catalog has it, so
 * installing a plugin again updates it.
 *
 * Fails with a message for a person when a plugin is not in the catalog, or when plugins need each other in a circle.
 *
 * - `id`: The plugin to make usable.
 * - `catalog`: The plugins the catalog lists, by id.
 * - `installed`: The plugins on this device, by id.
 * - `dependenciesOf`: The ids of the plugins that a catalog plugin depends on, from its `plugin.yml`.
 */
export async function installPlan(
  id: string,
  catalog: ReadonlyMap<string, CatalogPlugin>,
  installed: ReadonlyMap<string, InstalledPlugin>,
  dependenciesOf: (plugin: CatalogPlugin) => Promise<readonly string[]>,
): Promise<CatalogPlugin[]> {
  const plan: CatalogPlugin[] = [];
  const done = new Set<string>();

  const visit = async (needed: string, path: readonly string[]): Promise<void> => {
    if (done.has(needed)) return;
    if (path.includes(needed)) throw new Error(`These plugins need each other in a circle: ${[...path, needed].join(' → ')}.`);
    const here = installed.get(needed);
    if (here?.usable && needed !== id) return;

    const entry = catalog.get(needed);
    if (!here && !entry) {
      const by = path[path.length - 1];
      throw new Error(by ? `The plugin ${by} needs ${needed}, which is not in the catalog.` : `The plugin ${needed} is not in the catalog.`);
    }
    // A plugin that is installed says what it is missing; one that is not has to be read.
    const dependencies = here ? here.missingDependencies : await dependenciesOf(entry as CatalogPlugin);
    for (const dependency of dependencies) await visit(dependency, [...path, needed]);

    if (entry && (!here || needed === id)) plan.push(entry);
    done.add(needed);
  };

  await visit(id, []);
  return plan;
}
