'use client';

import { cn } from '@/client/lib/util';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faGripVertical } from '@fortawesome/sharp-light-svg-icons';
import { cloneElement, createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';

export interface DragContextType<T = unknown> {
  order: T[];
  mousePosition: { x: number; y: number };
  initialOffset: { x: number; y: number };
  selectedRef: { current: T | null };
  lockX: boolean;
  lockY: boolean;
  draggable?: boolean;
  setNewOrder: (items: T[]) => void;
  setInitialOrder: (items: T[]) => void;
  setInitialOffset?: (offset: { x: number; y: number }) => void;
  selectedElementRef?: { current: HTMLElement | null };
  requestDragStart?: (target: HTMLElement | null, e: PointerEvent) => void;
  cancelDragStart?: () => void;
}
const DragContext = createContext<DragContextType<any>>({
  order: [],
  mousePosition: { x: 0, y: 0 },
  initialOffset: { x: 0, y: 0 },
  selectedRef: { current: null },
  lockX: false,
  lockY: false,
  draggable: true,
  setNewOrder: _items => {},
  setInitialOrder: _items => {},
  setInitialOffset: _offset => {},
  selectedElementRef: { current: null },
  requestDragStart: () => {},
  cancelDragStart: () => {},
});

const DragItemContext = createContext({
  isDragging: { current: false },
  draggingIndex: -1,
  setDraggingIndex: (_idx: number) => {},
});

/**
 * A handle within a DragItem that can be used to grab and move the item.
 * This is useful for making the item draggable without having to click on the entire item.
 * It can be used to create a more user-friendly drag-and-drop experience.
 */
export function DragHandle({ className }: { className?: string }) {
  const { isDragging, setDraggingIndex } = useContext(DragItemContext);
  const { order, selectedRef, setInitialOffset, setInitialOrder, setNewOrder, requestDragStart, cancelDragStart } =
    useContext(DragContext);
  const { selectedElementRef } = useContext(DragContext);
  const dragItemRef = useRef<HTMLDivElement>(null);
  return (
    <div
      data-slot="drag-handle"
      className={cn('cursor-grab select-none active:cursor-grabbing', className)}
      onPointerDown={e => {
        // Start a delayed drag start request; will only begin a drag if held for 500ms.
        const dragItem = dragItemRef.current ?? (e.currentTarget.closest('[data-slot="drag-item"]') as HTMLDivElement);
        if (!dragItem) return;
        requestDragStart?.(dragItem, e.nativeEvent as PointerEvent);
      }}
      onPointerUp={_e => {
        // If pointer is released before the hold timer, cancel the pending drag.
        cancelDragStart?.();
      }}
      onPointerCancel={_e => {
        cancelDragStart?.();
      }}
    >
      <FontAwesomeIcon icon={faGripVertical} />
    </div>
  );
}
/**
 * A draggable item within a DragListContainer.
 */
export function DragItem({ children, className }: { children: React.ReactNode; className?: string }) {
  const { isDragging, draggingIndex, setDraggingIndex } = useContext(DragItemContext);
  const { mousePosition, initialOffset, lockX, lockY, order, selectedRef, selectedElementRef, draggable } =
    useContext(DragContext);
  const dragItemRef = useRef<HTMLDivElement>(null);

  // Find this item's index in the DOM on render
  const getIndex = () => {
    const dragItem = dragItemRef.current;
    const container = dragItem?.closest('[data-slot="drag-list-container"]');
    const children = container?.querySelectorAll('[data-slot="drag-item"]') ?? [];
    return Array.from(children).indexOf(dragItem as HTMLDivElement);
  };

  const clone = useMemo(() => {
    const index = getIndex();
    if (!draggable) return null;
    // Only clone the originally-selected item (selectedRef), not whatever item is under the mouse
    // Use the selected DOM element for matching the dragged item to avoid
    // relying on object identity of the items which can change during re-renders.
    const isSelected = !!selectedElementRef?.current && dragItemRef.current === selectedElementRef.current;
    if (!isDragging.current || !isSelected || !dragItemRef.current) return null;
    const rect = dragItemRef.current.getBoundingClientRect();
    // Use the initialOffset for positioning
    let top = mousePosition.y + window.scrollY - (initialOffset?.y ?? 0);
    let left = mousePosition.x + window.scrollX - (initialOffset?.x ?? 0);
    // If lockX or lockY is true, adjust the position accordingly
    if (lockX) left = rect.left + window.scrollX;
    if (lockY) top = rect.top + window.scrollY;
    const cloned = cloneElement(
      <div
        className="pointer-events-none absolute z-10 drop-shadow-2xl [body]:select-none"
        style={{
          left,
          top,
          width: rect.width,
          height: rect.height,
        }}
      >
        <div className={cn('relative', dragItemRef.current.className)}>{children}</div>
      </div>,
      {},
    );
    return cloned;
  }, [children, mousePosition, isDragging, initialOffset, order, selectedRef]);

  return (
    <DragItemContext.Provider
      value={{
        isDragging,
        draggingIndex,
        setDraggingIndex,
      }}
    >
      {isDragging.current &&
        (() => {
          const index = getIndex();
          const isSelected = !!selectedElementRef?.current && dragItemRef.current === selectedElementRef.current;
          return isSelected && clone ? createPortal(clone, document.body) : null;
        })()}
      <div data-slot="drag-item" className={cn('select-none', className)} ref={dragItemRef}>
        {children}
      </div>
    </DragItemContext.Provider>
  );
}
/**
 * A container that enables drag-and-drop reordering of its child DragItems.
 */
export function DragListContainer<T = unknown>({
  children,
  className,
  order,
  lockX = false,
  lockY = false,
  dragDelay = 0,
  draggable = true,
  onOrderChange,
}: {
  children: React.ReactNode;
  className?: string;
  order: T[];
  lockX?: boolean;
  lockY?: boolean;
  dragDelay?: number;
  draggable?: boolean;
  onOrderChange: (items: T[]) => void;
}) {
  const [mousePosition, setMousePosition] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const [initialOrder, setInitialOrder] = useState<T[]>(order);
  // This is the local display order during a drag. Updates while dragging will
  // change this local order, and the parent `onOrderChange` callback will
  // only be called on drag end to commit changes.
  const [displayOrder, setDisplayOrder] = useState<T[]>(order);
  const selectedRef = useRef<T | null>(null);
  const selectedElementRef = useRef<HTMLElement | null>(null);
  const isDragging = useRef(false);
  // Remove dragItemRef from here
  const [index, setIndex] = useState(0);
  const indexRef = useRef(index);
  const [draggingIndex, setDraggingIndex] = useState(-1);
  const [initialOffset, setInitialOffset] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const startDragTimerRef = useRef<number | null>(null);
  const startDragTargetRef = useRef<HTMLElement | null>(null);
  const isPointerDownRef = useRef(false);
  useEffect(() => {
    indexRef.current = index;
  }, [index]);

  // Update the local display order (not the parent) while dragging.
  const setNewOrder = useCallback((items: T[]) => setDisplayOrder(items), [setDisplayOrder]);

  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      setMousePosition({ x: e.clientX, y: e.clientY });
      if (!isDragging.current) return;
      const element = document.elementFromPoint(e.clientX, e.clientY)?.closest('[data-slot="drag-item"]');
      const dragContainer = element?.closest('[data-slot="drag-list-container"]');
      if (!element || !dragContainer) return;

      const dragItems = Array.from(dragContainer.querySelectorAll('[data-slot="drag-item"]'));
      const itemIndex = dragItems.indexOf(element);
      // Find the index of the currently selected element by DOM element rather
      // than relying on object identity (which may change during re-renders).
      const selectedIndex = selectedElementRef.current ? dragItems.indexOf(selectedElementRef.current) : -1;
      if (itemIndex === -1) return;

      const currentIndex = indexRef.current;
      if (itemIndex === currentIndex) return;

      // Build new order by removing the selected item at its current selectedIndex
      // in the DOM and inserting it at the hover position.
      const newOrder = [...displayOrder];
      if (selectedIndex !== -1) newOrder.splice(selectedIndex, 1);
      if (selectedRef.current !== null) newOrder.splice(itemIndex, 0, selectedRef.current);
      setNewOrder(newOrder);
      setIndex(itemIndex);
      setDraggingIndex(itemIndex);
      indexRef.current = itemIndex;
      // Do not overwrite `initialOrder` here. `initialOrder` should represent
      // the order at drag start so that Escape can revert to it. We set it in
      // handleDragStart.
    },
    [displayOrder, setNewOrder],
  );

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        isDragging.current = false;
        setDraggingIndex(-1);
        // Revert to the initial order on Escape without committing to parent.
        setDisplayOrder(initialOrder);
      }
    },
    [initialOrder, setDraggingIndex],
  );

  const clearStartDragTimer = useCallback(() => {
    if (startDragTimerRef.current !== null) {
      clearTimeout(startDragTimerRef.current as unknown as number);
      startDragTimerRef.current = null;
    }
  }, []);

  const requestDragStart = useCallback(
    (target: HTMLElement | null, e: PointerEvent) => {
      clearStartDragTimer();
      isPointerDownRef.current = true;
      startDragTargetRef.current = target;
      const timer = window.setTimeout(() => {
        // Don't start a drag if the pointer has already been released.
        if (!isPointerDownRef.current) return;
        const dragItem = startDragTargetRef.current ?? target;
        if (!dragItem) return;
        const dragContainer = dragItem?.closest('[data-slot="drag-list-container"]');
        const dragItems = Array.from(dragContainer?.querySelectorAll('[data-slot="drag-item"]') ?? []);
        const idx = dragItems.indexOf(dragItem as Element);
        isDragging.current = true;
        // Save the initial order at the start of this drag.
        setInitialOrder(displayOrder);
        selectedRef.current = displayOrder[idx] ?? null;
        selectedElementRef.current = dragItem as HTMLElement;
        setIndex(idx);
        setDraggingIndex(idx);
        indexRef.current = idx;
        // Store the offset between mouse and item
        const rect = (dragItem as HTMLElement)?.getBoundingClientRect?.();
        setInitialOffset(rect ? { x: e.clientX - rect.left, y: e.clientY - rect.top } : { x: 0, y: 0 });
        startDragTimerRef.current = null;
      }, dragDelay);
      startDragTimerRef.current = timer;
    },
    [clearStartDragTimer, displayOrder, setInitialOrder, setDraggingIndex, setIndex, setInitialOffset],
  );

  const cancelDragStart = useCallback(() => {
    // Cancel any pending drag start; this should be called on pointerup
    // if the timer hasn't elapsed yet.
    clearStartDragTimer();
    isPointerDownRef.current = false;
    startDragTargetRef.current = null;
  }, [clearStartDragTimer]);

  // Remove dragItemRef usage here
  const handleDragStart = useCallback(
    (e: PointerEvent) => {
      clearStartDragTimer();
      // Only start dragging if the pointer down is on a drag handle or drag item
      const target = e.target as HTMLElement;
      const dragItem = target.closest('[data-slot="drag-item"]');
      const hasDragHandle = dragItem?.querySelector('[data-slot="drag-handle"]') ?? false;
      // If the item has a drag handle, we expect the handle itself to call
      // `requestDragStart`. Otherwise, start a delayed request to begin
      // dragging only if the user holds the pointer for 500ms.
      if (dragItem && !hasDragHandle) {
        requestDragStart(dragItem as HTMLElement, e);
      }
      // Store the offset between mouse and item
      const rect = (dragItem as HTMLElement)?.getBoundingClientRect?.();
      setInitialOffset(rect ? { x: e.clientX - rect.left, y: e.clientY - rect.top } : { x: 0, y: 0 });
    },
    [
      displayOrder,
      setInitialOrder,
      setDraggingIndex,
      setIndex,
      setInitialOffset,
      requestDragStart,
      clearStartDragTimer,
    ],
  );

  const handleDragEnd = useCallback(() => {
    // Only commit the local display order to the parent if a drag actually occurred
    // (i.e. the display order differs from the initial order captured on drag start
    // and we were in a dragging state).
    if (!isDragging.current) return;
    isDragging.current = false;
    setDraggingIndex(-1);
    if (JSON.stringify(displayOrder) !== JSON.stringify(initialOrder)) {
      onOrderChange(displayOrder);
    }
    selectedRef.current = null;
    selectedElementRef.current = null;
    clearStartDragTimer();
  }, [displayOrder, initialOrder, onOrderChange]);

  // Keep the local display order in sync with parent `order` when not dragging.
  useEffect(() => {
    if (draggingIndex === -1) setDisplayOrder(order);
  }, [order, draggingIndex]);

  const handlePointerUpGlobal = useCallback(() => {
    cancelDragStart();
    handleDragEnd();
  }, [cancelDragStart, handleDragEnd]);

  useEffect(() => {
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('pointerdown', handleDragStart);
    window.addEventListener('pointerup', handlePointerUpGlobal);
    window.addEventListener('pointercancel', cancelDragStart);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('pointerdown', handleDragStart);
      window.removeEventListener('pointerup', handlePointerUpGlobal);
      window.removeEventListener('pointercancel', cancelDragStart);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [handleMouseMove, handleDragStart, handlePointerUpGlobal, handleKeyDown, cancelDragStart]);

  return (
    <DragContext.Provider
      value={{
        order: displayOrder,
        lockX,
        lockY,
        mousePosition,
        draggable,
        setNewOrder,
        setInitialOrder,
        selectedRef,
        initialOffset,
        setInitialOffset,
        selectedElementRef,
        requestDragStart,
        cancelDragStart,
      }}
    >
      <DragItemContext.Provider
        value={{
          isDragging,
          draggingIndex,
          setDraggingIndex,
        }}
      >
        <div data-slot="drag-list-container" className={className}>
          {children}
        </div>
      </DragItemContext.Provider>
    </DragContext.Provider>
  );
}
