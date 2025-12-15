'use client';

import { cn, keySelect } from '@/client/lib/util';
import { Button } from '@/client/ui/button';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faTimes } from '@fortawesome/sharp-light-svg-icons';
import {
  Children,
  createContext,
  isValidElement,
  memo,
  ReactElement,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

export interface TabsProps {
  /** The children of the Tabs component. */
  children: ReactNode;
  /** The width of the tab labels. */
  type?: 'compact' | 'full';
  /** The index of the active tab. */
  activeTab?: number;
  /** Triggered when the active tab changes. */
  onActiveTabChange?: (index: number) => void;
  /** Additional class names for the Tabs component. */
  className?: string;
  /** Maximum number of tabs to keep cached in DOM. Default is 3. */
  maxCachedTabs?: number;
  /** Whether tabs are closeable. */
  closeable?: boolean;
  /** Callback when a tab is closed. */
  onClose?: (index: number) => void;
}

export interface TabItemProps {
  children: ReactNode;
}

const TabsContext = createContext({
  activeTabIdx: 0,
  closeable: false,
  setActiveTabIdx: (_: number) => {},
  onClose: (_: number) => {},
});

// Helper to check component type by displayName (survives HMR)
function isComponentType(element: ReactElement, name: string): boolean {
  const type = element.type as any;
  // Check displayName directly
  if (type?.displayName === name) return true;
  // Check function name
  if (type?.name === name) return true;
  // Check for memo-wrapped components (type.type is the inner component)
  if (type?.type?.displayName === name) return true;
  if (type?.type?.name === name) return true;
  // Check render property for forwardRef/memo
  if (type?.render?.displayName === name) return true;
  if (type?.render?.name === name) return true;
  return false;
}

/**
 * A component that renders a set of tabs.
 * Uses LRU caching to keep only recently viewed tabs in the DOM for memory efficiency.
 * @example
 * <Tabs>
 *  <TabItem>
 *    <TabLabel>Tab 1</TabLabel>
 *    <TabContent>Content 1</TabContent>
 *  </TabItem>
 *  <TabItem>
 *    <TabLabel>Tab 2</TabLabel>
 *    <TabContent>Content 2</TabContent>
 *  </TabItem>
 * </Tabs>
 */
const emptyFn = () => {};

export function Tabs({
  children,
  type = 'compact',
  activeTab = 0,
  onActiveTabChange,
  className,
  closeable = false,
  onClose,
  maxCachedTabs = 3,
}: TabsProps) {
  const [activeTabIdx, setActiveTabIdxState] = useState(activeTab);
  // Track recently used tabs in order (most recent first)
  const [cachedTabs, setCachedTabs] = useState<number[]>([activeTab]);
  const prevActiveRef = useRef(activeTab);

  // Use refs to avoid recreating callbacks when props change
  const onActiveTabChangeRef = useRef(onActiveTabChange);
  onActiveTabChangeRef.current = onActiveTabChange;

  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  const setActiveTabIdx = useCallback((idx: number) => {
    setActiveTabIdxState(idx);
    onActiveTabChangeRef.current?.(idx);
  }, []);

  const handleClose = useCallback((idx: number) => {
    onCloseRef.current?.(idx);
  }, []);

  // Update cached tabs when active tab changes (LRU strategy)
  useEffect(() => {
    if (prevActiveRef.current !== activeTabIdx) {
      prevActiveRef.current = activeTabIdx;
      setCachedTabs(prev => {
        // Remove the tab if it's already in the cache
        const filtered = prev.filter(t => t !== activeTabIdx);
        // Add to the front (most recently used)
        const updated = [activeTabIdx, ...filtered];
        // Keep only maxCachedTabs
        return updated.slice(0, maxCachedTabs);
      });
    }
  }, [activeTabIdx, maxCachedTabs]);

  const labels = useMemo(
    () =>
      Children.toArray(children)
        .map((child, idx) => {
          if (isValidElement(child) && isComponentType(child, 'TabItem')) {
            const label = Children.toArray((child as ReactElement<TabItemProps>).props.children).find(
              lbl => isValidElement(lbl) && isComponentType(lbl, 'TabLabel'),
            );
            if (isValidElement(label)) {
              const labelProps = (label as ReactElement<{ children: ReactNode }>).props;
              return (
                <TabLabel key={idx} tabIndex={idx}>
                  {labelProps.children}
                </TabLabel>
              );
            }
            return null;
          }
          return null;
        })
        .filter(Boolean),
    [children],
  );

  // Only render cached tabs in DOM, show/hide with CSS for performance
  const cachedTabContents = useMemo(
    () =>
      Children.toArray(children).map((child, idx) => {
        // Only render if tab is in cache
        if (!cachedTabs.includes(idx)) return null;

        if (isValidElement(child) && isComponentType(child, 'TabItem')) {
          const content = Children.toArray((child as ReactElement<TabItemProps>).props.children).find(
            lbl => isValidElement(lbl) && isComponentType(lbl, 'TabContent'),
          );
          if (isValidElement(content)) {
            return (
              <div
                key={idx}
                className={cn('min-h-0 grow flex-col', idx === activeTabIdx ? 'flex' : 'hidden')}
                data-tab-index={idx}
              >
                {content}
              </div>
            );
          }
        }
        return null;
      }),
    [children, activeTabIdx, cachedTabs],
  );

  const tabLabel = cn('flex w-full', {
    // 'justify-between': type === 'compact',
    // 'justify-around': type === 'full',
    // 'border-b-1 border-primary': type === 'compact',
  });

  const contextValue = useMemo(
    () => ({ activeTabIdx, setActiveTabIdx, closeable, onClose: handleClose }),
    [activeTabIdx, setActiveTabIdx, closeable, handleClose],
  );

  return (
    <TabsContext.Provider value={contextValue}>
      <div data-name="tabs" className={cn('flex min-h-0 w-full flex-col', className)}>
        <div data-name="tab-labels" className={tabLabel}>
          {labels}
        </div>
        <div data-name="active-tab-content" className="flex min-h-0 grow flex-col">
          {cachedTabContents}
        </div>
      </div>
    </TabsContext.Provider>
  );
}
/**
 * A tab item that wraps a label and its content.
 * @example
 * <TabItem>
 *  <TabLabel>Tab 1</TabLabel>
 *  <TabContent>Content 1</TabContent>
 * </TabItem>
 */
export function TabItem({}: TabItemProps) {
  return <></>;
}
TabItem.displayName = 'TabItem';
/**
 * A tab label that is displayed in the tab bar.
 */
export const TabLabel = memo(function TabLabel({
  children,
  tabIndex = 0,
  className,
}: {
  children: ReactNode;
  tabIndex?: number;
  className?: string;
}) {
  const { setActiveTabIdx, activeTabIdx, closeable, onClose } = useContext(TabsContext);

  return (
    <div
      data-name="tab-label"
      className={cn(
        'flex cursor-pointer items-center justify-center gap-2 p-2 text-center',
        {
          'bg-primary hover:bg-primary font-semibold text-white': tabIndex === activeTabIdx,
        },
        className,
      )}
      tabIndex={0}
      onClick={() => setActiveTabIdx(tabIndex)}
      onKeyDown={e => keySelect(e) && setActiveTabIdx(tabIndex)}
    >
      <div className="whitespace-nowrap">{children}</div>
      {closeable && (
        <Button
          onClick={e => {
            e.stopPropagation();
            onClose?.(tabIndex);
          }}
          aria-label="Close tab"
          aspect="square"
          variant="icon"
        >
          <FontAwesomeIcon icon={faTimes} />
        </Button>
      )}
    </div>
  );
});
TabLabel.displayName = 'TabLabel';
/**
 * The content of a tab that is displayed when the tab is active.
 */
export function TabContent({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div data-name="tab-content" className={cn('min-h-0 grow overflow-auto', className)}>
      {children}
    </div>
  );
}
TabContent.displayName = 'TabContent';
