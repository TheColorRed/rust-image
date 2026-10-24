/** Anything a root can pass through or end in: a tab (a section of the editor), or a group in one. */
export type Place = { key: string; label: string; kind?: string; controls: Place[] };

/** Makes what a root names that is not there yet. */
export type PlaceMaker = {
  /** A tab with this name. */
  section: (label: string) => Place;
  /** A group with this name, inside the place with the key `parentKey`. */
  group: (label: string, parentKey: string) => Place;
};

const same = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();

/**
 * Finds where the controls of a plugin go. `root` is the name of a tab and then the names of the groups inside it, like
 * `Beauty/Makeup`, and is matched to the names the person sees, ignoring case. A tab or group that is not there is made (and put last), so
 * a plugin can add a tab of its own or a group of its own. Only a group can be gone into: a root that names something else on the way
 * is an error.
 *
 * Returns the list the controls go in, and `undo`, which takes away the tabs and groups this made once they are empty, so a plugin that
 * is taken away leaves no empty tab behind.
 *
 * - `sections`: The editor's tabs. A tab that is made is added to it.
 * - `root`: The path of names, separated by `/`.
 * - `make`: How to make a tab or a group that is missing.
 */
export function resolveRoot(sections: Place[], root: string, make: PlaceMaker): { list: Place[]; undo: () => void } {
  const names = root.split('/').map(name => name.trim());
  if (names.some(name => name === '')) throw new Error(`"${root}" is not a root: write the tab and the groups it goes in, like Beauty/Makeup.`);
  const made: { from: Place[]; place: Place }[] = [];

  let list = sections;
  let parentKey = '';
  names.forEach((name, depth) => {
    let place = list.find(candidate => same(candidate.label, name));
    if (!place) {
      place = depth === 0 ? make.section(name) : make.group(name, parentKey);
      list.push(place);
      made.push({ from: list, place });
    } else if (depth > 0 && place.kind !== 'group') {
      throw new Error(`"${root}" goes into ${name}, which is not a group.`);
    }
    list = place.controls;
    parentKey = place.key;
  });

  return {
    list,
    undo: () => {
      // The innermost first, so a group that held only another made group is empty by the time it is looked at.
      for (const { from, place } of [...made].reverse()) {
        const at = from.indexOf(place);
        if (at >= 0 && place.controls.length === 0) from.splice(at, 1);
      }
    },
  };
}
