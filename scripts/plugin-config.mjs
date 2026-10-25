// What the plugin scripts need to agree on about a plugin's folder.
import path from 'node:path';

/** The fixed location for a plugin's built scripts; each plugin keeps its output separate. */
export function outDirOf(pluginDir) {
  return path.join(pluginDir, 'dist');
}
