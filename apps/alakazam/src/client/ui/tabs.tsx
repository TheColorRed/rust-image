'use client';

import { cn, focusable, keySelect } from '@/client/lib/util';
import { Button } from '@/client/ui/button';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faTimes } from '@fortawesome/sharp-light-svg-icons';
import {
  Children,
  createContext,
  createElement,
  isValidElement,
  ReactElement,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';

export type TabClose = (index: number, id: string | number | null) => void;
export type TabActiveChange = (index: number, id: string | number | null) => void;

export interface TabsProps {
  /** The children of the Tabs component. */
  children: ReactNode;
  /** The width of the tab labels. */
  type?: 'compact' | 'full';
  /** The index of the active tab. */
  activeTab?: number;
  /** The id of the active tab. */
  activeId?: string | number | null;
  /** Whether tabs are closeable. */
  closeable?: boolean;
  /** Additional class names for the Tabs component. */
  className?: string;
  /** Triggered when the active tab changes passing the new id. */
  onActiveTabChange?: (index: number, id: string | number | null) => void;
  /** Triggered when a tab is closed passing the closed tab index. */
  onClose?: (index: number, id: string | number | null) => void;
}

export interface TabItemProps {
  id?: string | number | null;
  children: ReactNode;
}

const TabsContext = createContext({
  activeTabState: [0, null] as [number, string | number | null],
  closeable: false,
  setActiveTabState: (_: [number, string | number | null]) => {},
  onClose: (_idx: number, _id: string | number | null) => {},
});

const TabItemContext = createContext({
  id: null as string | number | null | undefined,
});
/**
 * A component that renders a set of tabs.
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
export function Tabs({
  children,
  activeTab = 0,
  activeId,
  className,
  closeable = false,
  onActiveTabChange,
  onClose,
}: TabsProps) {
  const [activeTabState, setActiveTabState] = useState<[number, string | number | null]>([activeTab, null]);

  useEffect(() => {
    if (activeId !== undefined) {
      const idx = Children.toArray(children).findIndex(child => {
        if (isValidElement(child) && child.type === TabItem) {
          return (child as ReactElement<TabItemProps>).props.id === activeId;
        }
        return false;
      });
      if (idx !== -1) {
        setActiveTabState([idx, activeId]);
      }
    } else {
      setActiveTabState([activeTab, null]);
    }
  }, [activeTab, activeId, children]);

  useEffect(() => {
    onActiveTabChange?.(activeTabState[0], activeTabState[1]);
  }, [activeTabState]);

  const handleClose = useCallback((idx: number, id: string | number | null) => {
    onClose?.(idx, id);
  }, []);

  const labels = Children.toArray(children)
    .map((child, idx) => {
      if (isValidElement(child) && child.type === TabItem) {
        const label = Children.toArray((child as ReactElement<TabItemProps>).props.children).find(
          lbl => isValidElement(lbl) && lbl.type === TabLabel,
        );
        return isValidElement(label)
          ? createElement(
              (label as ReactElement<typeof TabLabel>).type,
              { key: idx, tabIndex: idx, id: (child as ReactElement<TabItemProps>).props.id },
              (label as ReactElement<{ children: ReactNode }>).props.children,
            )
          : null;
      }
      return null;
    })
    .filter(Boolean);

  const activeTabContent = Children.toArray(children).map((child, idx) => {
    if (isValidElement(child) && child.type === TabItem) {
      const content = Children.toArray((child as ReactElement<TabItemProps>).props.children).find(
        lbl => isValidElement(lbl) && lbl.type === TabContent,
      );
      return isValidElement(content) && idx === activeTabState[0] ? content : null;
    }
    return null;
  });

  const tabLabel = cn('flex w-full', {
    // 'justify-between': type === 'compact',
    // 'justify-around': type === 'full',
    // 'border-b-6 border-primary': type === 'compact',
    // 'border-b-0': type === 'full',
  });

  const contextValue = useMemo(
    () => ({ activeTabState, setActiveTabState, closeable, onClose: handleClose }),
    [activeTabState, setActiveTabState, closeable, handleClose],
  );

  return (
    <TabsContext.Provider value={contextValue}>
      <div data-name="tabs" className={cn('flex min-h-0 w-full flex-col', className)}>
        <div data-name="tab-labels" className={tabLabel}>
          {labels}
        </div>
        <div data-name="active-tab-content" className="flex min-h-0 grow flex-col">
          {activeTabContent}
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
export function TabItem({ id, children }: TabItemProps) {
  return <TabItemContext.Provider value={{ id }}>{children}</TabItemContext.Provider>;
}
/**
 * A tab label that is displayed in the tab bar.
 */
export function TabLabel({
  children,
  tabIndex = 0,
  id: propId,
  className,
}: {
  children: ReactNode;
  tabIndex?: number;
  id?: string | number | null;
  className?: string;
}) {
  const { setActiveTabState, activeTabState, closeable, onClose } = useContext(TabsContext);
  const { id: contextId } = useContext(TabItemContext);
  const id = propId ?? contextId;

  return (
    <div
      data-name="tab-label"
      className={cn(
        'flex cursor-pointer items-center justify-center gap-2 p-2 text-center',
        {
          'bg-primary hover:bg-primary font-semibold text-white': tabIndex === activeTabState[0],
        },
        className,
        focusable,
      )}
      tabIndex={0}
      onClick={() => setActiveTabState([tabIndex, id ?? null])}
      onKeyDown={e => keySelect(e) && setActiveTabState([tabIndex, id ?? null])}
    >
      <div className="whitespace-nowrap">{children}</div>
      {closeable && (
        <Button
          onClick={e => {
            e.stopPropagation();
            onClose?.(tabIndex, id ?? null);
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
}
/**
 * The content of a tab that is displayed when the tab is active.
 */
export function TabContent({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div data-name="tab-content" className={className}>
      {children}
    </div>
  );
}
