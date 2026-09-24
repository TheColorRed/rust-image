'use client';

import { cn } from '@/lib/util';
import { Tooltip } from '@/ui/tooltip';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faChevronDown,
  faChevronRight,
  faFile,
  faFolder,
  faFolderOpen,
  faSquareMinus,
  faSquarePlus,
} from '@fortawesome/sharp-light-svg-icons';
import React, { createContext, useCallback, useContext, useMemo, useState } from 'react';

type TreeContextValue = {
  expandedSet: Set<string>;
  toggleExpanded: (id: string) => void;
  /** Expand multiple ids (add to expanded set) */
  expandMany: (ids: string[]) => void;
  /** Collapse multiple ids (remove from expanded set) */
  collapseMany: (ids: string[]) => void;
  selectedId: string | null;
  onSelect?: (id: string | null) => void;
  indent: number;
};

const TreeContext = createContext<TreeContextValue | null>(null);

export interface TreeProps {
  children: React.ReactNode;
  /** Controlled expanded node ids */
  expandedIds?: string[];
  defaultExpandedIds?: string[];
  onExpandedChange?: (ids: string[]) => void;
  selectedId?: string | null;
  onSelect?: (id: string | null) => void;
  className?: string;
  indent?: number;
}

export interface TreeItemProps {
  id: string;
  label?: React.ReactNode;
  icon?: React.ReactNode;
  children?: React.ReactNode;
  className?: string;
  /** Default expanded state for this item; when true the item will be expanded by default */
  expanded?: boolean;
  depth?: number; // internal use
}

/**
 * Tree: container for a set of `Tree.Item`s. Use `Tree.Item` as children to define the structure.
 * Example:
 * <Tree>
 *   <Tree.Item id="1" label="Root">
 *     <Tree.Item id="1-1" label="Child" />
 *   </Tree.Item>
 * </Tree>
 */
export function Tree({
  children,
  expandedIds,
  defaultExpandedIds = [],
  onExpandedChange,
  selectedId: selectedIdProp,
  onSelect,
  className,
  indent = 16,
}: TreeProps) {
  const collectDefaultExpandedIds = (nodes: React.ReactNode): string[] => {
    const out: string[] = [];
    React.Children.forEach(nodes, child => {
      if (!React.isValidElement(child)) return;
      const props: any = child.props || {};
      if (typeof props.id === 'string' && props.expanded) out.push(props.id);
      if (props.children) out.push(...collectDefaultExpandedIds(props.children));
    });
    return out;
  };

  const [expandedSetInternal, setExpandedSetInternal] = useState<Set<string>>(() => {
    const idsFromItems = collectDefaultExpandedIds(children);
    return new Set([...defaultExpandedIds, ...idsFromItems]);
  });

  const expandedSet = useMemo(
    () => new Set(expandedIds ?? Array.from(expandedSetInternal)),
    [expandedIds, expandedSetInternal],
  );
  const isControlled = expandedIds !== undefined;

  // Compute a stable signature of the tree items that declare `expanded` so we
  // only resync the internal expanded set when the *structure* (or explicit
  // per-item `expanded` props) actually changes. `children` will be a new
  // React node on every render, so using it directly in the effect deps caused
  // spurious resets when nothing meaningful changed.
  const itemsSignature = React.useMemo(() => {
    const idsFromItems = collectDefaultExpandedIds(children);
    idsFromItems.sort();
    return idsFromItems.join('|');
  }, [children]);

  // Keep internal expanded set in sync with per-item `expanded` props and
  // `defaultExpandedIds`. Only run when the `itemsSignature` or
  // `defaultExpandedIds` change (or when the tree is controlled).
  React.useEffect(() => {
    if (isControlled) return;
    const idsFromItems = itemsSignature ? itemsSignature.split('|').filter(Boolean) : [];
    const nextSet = new Set<string>([...defaultExpandedIds, ...idsFromItems]);

    // Shallow compare sets by size and membership
    const curr = expandedSetInternal;
    let equal = curr.size === nextSet.size;
    if (equal) {
      for (const id of nextSet) {
        if (!curr.has(id)) {
          equal = false;
          break;
        }
      }
    }

    if (!equal) {
      setExpandedSetInternal(nextSet);
    }
    // We deliberately do NOT include `expandedSetInternal` in the deps array so
    // user-driven expansion toggles (which change the internal set) don't
    // immediately trigger a sync that would reset the user's changes.
  }, [itemsSignature, defaultExpandedIds, isControlled]);

  const toggleExpanded = useCallback(
    (id: string) => {
      const next = new Set(expandedSet);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      if (!isControlled) setExpandedSetInternal(next);
      onExpandedChange?.(Array.from(next));
    },
    [expandedSet, isControlled, onExpandedChange],
  );

  const [selectedId, setSelectedId] = useState<string | null>(selectedIdProp ?? null);
  React.useEffect(() => {
    if (selectedIdProp !== undefined) setSelectedId(selectedIdProp);
  }, [selectedIdProp]);

  const handleSelect = useCallback(
    (id: string | null) => {
      if (selectedIdProp === undefined) setSelectedId(id);
      onSelect?.(id);
    },
    [onSelect, selectedIdProp],
  );

  const expandMany = useCallback(
    (ids: string[]) => {
      const next = new Set(expandedSet);
      ids.forEach(id => next.add(id));
      if (!isControlled) setExpandedSetInternal(next);
      onExpandedChange?.(Array.from(next));
    },
    [expandedSet, isControlled, onExpandedChange],
  );

  const collapseMany = useCallback(
    (ids: string[]) => {
      const next = new Set(expandedSet);
      ids.forEach(id => next.delete(id));
      if (!isControlled) setExpandedSetInternal(next);
      onExpandedChange?.(Array.from(next));
    },
    [expandedSet, isControlled, onExpandedChange],
  );

  const ctx = useMemo<TreeContextValue>(
    () => ({ expandedSet, toggleExpanded, expandMany, collapseMany, selectedId, onSelect: handleSelect, indent }),
    [expandedSet, toggleExpanded, expandMany, collapseMany, selectedId, handleSelect, indent],
  );

  return (
    <TreeContext.Provider value={ctx}>
      <div role="tree" className={cn('text-white select-none', className)}>
        {children}
      </div>
    </TreeContext.Provider>
  );
}

export function TreeItem({ id, label, icon, children, className, depth = 0 }: TreeItemProps) {
  const ctx = useContext(TreeContext);
  if (!ctx) throw new Error('TreeItem must be used inside a Tree');

  const { expandedSet, toggleExpanded, expandMany, collapseMany, selectedId, onSelect, indent } = ctx;
  const hasChildren = React.Children.toArray(children).length > 0;
  const expanded = expandedSet.has(id);

  const collectIds = (nodes: React.ReactNode): string[] => {
    const out: string[] = [];
    React.Children.forEach(nodes, child => {
      if (!React.isValidElement(child)) return;
      const props: any = child.props || {};
      if (typeof props.id === 'string') {
        out.push(props.id);
        out.push(...collectIds(props.children));
      }
    });
    return out;
  };

  const descendantIds = React.useMemo(() => collectIds(children), [children]);
  const idsToToggle = React.useMemo(() => [id, ...descendantIds], [id, descendantIds]);
  const allExpanded = idsToToggle.length > 0 && idsToToggle.every(i => expandedSet.has(i));

  const handleToggle = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (hasChildren) toggleExpanded(id);
  };
  const handleSelect = (e: React.MouseEvent | React.KeyboardEvent) => {
    e.stopPropagation();
    onSelect?.(id);
  };

  const handleExpandAllClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (allExpanded) collapseMany(idsToToggle);
    else expandMany(idsToToggle);
  };

  const handleExpandAllKey = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      e.stopPropagation();
      if (allExpanded) collapseMany(idsToToggle);
      else expandMany(idsToToggle);
    }
  };

  return (
    <div>
      <div
        role="treeitem"
        aria-expanded={hasChildren ? expanded : undefined}
        tabIndex={0}
        onClick={handleSelect}
        onDoubleClick={(e: React.MouseEvent) => {
          e.stopPropagation();
          if (hasChildren) toggleExpanded(id);
        }}
        onKeyDown={(e: React.KeyboardEvent) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            handleSelect(e);
          } else if (e.key === 'ArrowRight' && hasChildren && !expanded) {
            e.preventDefault();
            toggleExpanded(id);
          } else if (e.key === 'ArrowLeft' && hasChildren && expanded) {
            e.preventDefault();
            toggleExpanded(id);
          }
        }}
        className={cn(
          'group flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1 hover:bg-white/5',
          selectedId === id ? 'bg-white/10 font-semibold' : '',
          className,
        )}
        style={{ paddingLeft: depth * indent }}
      >
        <div
          onClick={handleToggle}
          onDoubleClick={(e: React.MouseEvent) => e.stopPropagation()}
          className={cn(
            'flex h-4 w-4 cursor-pointer items-center justify-center text-neutral-400 transition-colors hover:text-white',
          )}
        >
          {hasChildren ? (
            expanded ? (
              <FontAwesomeIcon icon={faChevronDown} size="sm" />
            ) : (
              <FontAwesomeIcon icon={faChevronRight} size="sm" />
            )
          ) : (
            <span className="w-4" />
          )}
        </div>

        <div className="flex w-5 items-center justify-center text-neutral-300">
          {icon ??
            (hasChildren ? (
              expanded ? (
                <FontAwesomeIcon icon={faFolderOpen} />
              ) : (
                <FontAwesomeIcon icon={faFolder} />
              )
            ) : (
              <FontAwesomeIcon icon={faFile} />
            ))}
        </div>

        <div className="flex-1 truncate">{label}</div>

        {descendantIds.length > 0 && (
          <Tooltip content={allExpanded ? 'Collapse all' : 'Expand all'} position="below">
            <button
              onClick={handleExpandAllClick}
              onKeyDown={handleExpandAllKey}
              aria-label={allExpanded ? 'Collapse all children' : 'Expand all children'}
              className="invisible ml-2 inline-grid h-7 w-7 shrink-0 cursor-pointer place-items-center overflow-hidden rounded p-0 text-neutral-400 opacity-0 transition-colors group-hover:visible group-hover:opacity-100 hover:bg-white/10 hover:text-white"
              style={{ padding: 0, margin: 0 }}
            >
              <FontAwesomeIcon icon={allExpanded ? faSquareMinus : faSquarePlus} className="m-0" />
            </button>
          </Tooltip>
        )}
      </div>

      {hasChildren && expanded && (
        <div role="group">
          {React.Children.map(children, child =>
            React.isValidElement(child)
              ? React.cloneElement(child as React.ReactElement<TreeItemProps>, { depth: depth + 1 })
              : child,
          )}
        </div>
      )}
    </div>
  );
}

/*
  ExampleTree: small interactive demo to make it easy to test locally.
  - Renders a small sample tree
  - Shows selection and expansion behavior
*/
export function ExampleTree() {
  const [selected, setSelected] = useState<string | null>(null);

  return (
    <div className="bg-default rounded-md p-2">
      <Tree selectedId={selected} onSelect={id => setSelected(id)}>
        <TreeItem id="rgb" label="RGB" expanded>
          <TreeItem id="rgb-r" label="Red" />
          <TreeItem id="rgb-g" label="Green" />
          <TreeItem id="rgb-b" label="Blue" />
        </TreeItem>

        <TreeItem id="cmyk" label="CMYK">
          <TreeItem id="cmyk-light" label="Light">
            <TreeItem id="cmyk-light-pastel" label="Pastel" />
            <TreeItem id="cmyk-light-pure" label="Pure" />
          </TreeItem>
          <TreeItem id="cmyk-grayscale" label="Grayscale" />
        </TreeItem>

        <TreeItem id="layers" label="Layers" />
      </Tree>

      <div className="mt-2 text-neutral-300">Selected: {selected ?? 'None'}</div>
    </div>
  );
}

export default Tree;
