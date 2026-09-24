import { swatches, SwatchesType } from '@/data/color-swatches';
import { fuzzySearch } from '@/lib/strings';
import { ActiveColor, activeColorState$, backgroundColor, foregroundColor } from '@/state/color';
import { Input } from '@/ui/input';
import Tree, { TreeItem } from '@/ui/tree';
import { useCallback, useEffect, useMemo, useState } from 'react';

function Colors({ colors }: { colors: string[] }) {
  const [activeColor, setActiveColor] = useState<ActiveColor | null>(null);

  const handleClick = useCallback(
    (color: string) => {
      const action = activeColor === 'fg' ? foregroundColor : backgroundColor;
      action.next(color);
    },
    [activeColor],
  );

  useEffect(() => {
    const activeColorSub = activeColorState$.subscribe(setActiveColor);
    return () => activeColorSub.unsubscribe();
  }, []);

  return (
    <div className="bg-medium flex flex-wrap gap-1 p-2">
      {colors.map((color, index) => (
        <button
          key={index}
          style={{ backgroundColor: color }}
          onClick={() => handleClick(color)}
          className="border-dark h-8 w-8 cursor-pointer border"
        />
      ))}
    </div>
  );
}

export function Swatches() {
  const [search, setSearch] = useState<string>('');
  const [expandedIds, setExpandedIds] = useState<string[]>([]);

  const filteredSwatches = useMemo(() => {
    if (!search) return swatches;
    // Do a deep filter of the swatches object using a "fuzzy" search on keys and keywords
    const result: SwatchesType = {};
    for (const [categoryKey, groups] of Object.entries(swatches)) {
      const matchedGroups: SwatchesType[string] = {};
      for (const [groupKey, group] of Object.entries(groups)) {
        // Check if group key matches search
        if (fuzzySearch(search, groupKey)) {
          matchedGroups[groupKey] = group;
          continue;
        }
        // Check if any keywords match search
        if (group.keywords.some(keyword => fuzzySearch(search, keyword))) {
          matchedGroups[groupKey] = group;
          continue;
        }
      }
      if (Object.keys(matchedGroups).length > 0) {
        result[categoryKey] = matchedGroups;
      }
    }
    return result;
  }, [search]);

  // When search changes, auto-expand all items matching the search
  const controlledExpandedIds = useMemo(() => {
    if (!search) return expandedIds; // let user control expansion when not searching
    // Auto-expand all items when searching
    const allIds: string[] = [];
    for (const categoryKey of Object.keys(filteredSwatches)) {
      allIds.push(`category-${categoryKey}`);
      for (const groupKey of Object.keys(filteredSwatches[categoryKey])) {
        allIds.push(`group-${categoryKey}-${groupKey}`);
      }
    }
    return allIds;
  }, [search, filteredSwatches, expandedIds]);

  const handleExpandedChange = useCallback((ids: string[]) => {
    setExpandedIds(ids);
  }, []);

  return (
    <div className="max-h-80 min-h-80 overflow-y-auto">
      <Input
        placeholder="Search swatches..."
        className="m-4 w-auto"
        value={search}
        onChange={value => setSearch(value)}
      />
      <Tree expandedIds={controlledExpandedIds} onExpandedChange={handleExpandedChange}>
        {Object.entries(filteredSwatches).map(([categoryKey, groups]) => (
          <TreeItem key={`category-${categoryKey}`} id={`category-${categoryKey}`} label={categoryKey}>
            {Object.entries(groups).map(([key, group]) => (
              <TreeItem key={`group-${categoryKey}-${key}`} id={`group-${categoryKey}-${key}`} label={key}>
                <Colors colors={group.items} />
              </TreeItem>
            ))}
          </TreeItem>
        ))}
      </Tree>
    </div>
  );
}
