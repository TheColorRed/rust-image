'use client';

import { useOverlayPosition } from '@/hooks/overlay-position';
import { cn, focusable } from '@/lib/util';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown, faChevronUp } from '@fortawesome/sharp-light-svg-icons';
import React, {
  Children,
  cloneElement,
  createContext,
  isValidElement,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { createPortal } from 'react-dom';
import { Option, OptionProps } from './option';

type SelectValue = string | number;

export interface SelectProps<T extends SelectValue = SelectValue> {
  value?: T;
  disabled?: boolean;
  placeholder?: string;
  className?: string;
  children: React.ReactNode;
  size?: 'sm' | 'md' | 'lg';
  onSelect?: (value: T) => void;
}

export interface SelectContextValue<T = unknown> {
  rect: DOMRect;
  element?: HTMLElement | null;
  value?: T;
  label?: string | ReactNode;
  close?: () => void;
}

const SelectContext = createContext<SelectContextValue>({
  rect:
    typeof window !== 'undefined'
      ? new DOMRect()
      : ({ left: 0, top: 0, width: 0, height: 0, right: 0, bottom: 0 } as DOMRect),
  value: '',
  label: '',
  element: null,
  close: undefined,
});

export function Select<T extends string | number>({
  value: propValue,
  className,
  disabled,
  children,
  placeholder,
  size = 'md',
  onSelect,
}: SelectProps<T>) {
  const wrapperRef = useRef<HTMLButtonElement>(null);
  const [value, setValue] = useState<T | undefined>(propValue);
  const hiddenValueRef = useRef<HTMLInputElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const isControlled = propValue !== undefined;
  const currentValue = isControlled ? (propValue as T | undefined) : value;
  const lastNavRef = useRef<number>(0);
  // const isOpen = useRef(false);
  // const toggle = () => (isOpen.current = !isOpen.current);
  const toggle = () => setIsOpen(v => !v);

  // Find the selected option
  const selectedOption = useMemo(
    () =>
      Children.toArray(children).find(
        child => isValidElement<{ value: T }>(child) && child.props.value === currentValue,
      ),
    [children, currentValue],
  );
  const baseClasses = cn(
    // 'appearance-none',
    // Cursor
    'cursor-pointer',
    // Sizing
    'w-full',
    // Layout
    'flex justify-between items-center',
    // Border
    'border border-gray-300 bg-medium rounded-md py-2 px-3',
    size === 'sm' && 'text-sm py-1',
    size === 'md' && 'text-base py-1',
    size === 'lg' && 'text-lg py-3',
    // Accessibility (blue outline 2px with 2px offset)
    focusable,
  );

  useEffect(() => {
    // Keep internal state in sync if controlled
    setValue(propValue as T);
  }, [propValue]);

  const label = useMemo(() => {
    if (React.isValidElement<{ children: ReactNode }>(selectedOption) && 'props' in selectedOption) {
      return selectedOption.props.children;
    }
    return undefined;
  }, [selectedOption]);

  const onItemChange = useCallback(
    (v: T) => {
      // If uncontrolled, update local state. If controlled, parent owns the value.
      if (!isControlled) setValue(v);
      onSelect?.(v);
    },
    [isControlled, onSelect],
  );

  useEffect(() => {
    const handleArrowNavigation = (event: KeyboardEvent) => {
      if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
      // If the select is not focused (or focus is inside the button), ignore
      if (!wrapperRef.current || !wrapperRef.current.contains(document.activeElement)) return;

      event.preventDefault();
      const options = Children.toArray(children).filter(
        child =>
          isValidElement<OptionProps<T>>(child) &&
          child.type === Option &&
          (child.props as OptionProps<T>).type !== 'separator',
      );
      if (!options || options.length === 0) return;

      let currentIndex = options.findIndex(option => {
        if (!isValidElement<OptionProps<T>>(option) || option.type !== Option) return false;
        return option.props.value === currentValue;
      });

      // If current selection is not in the filtered options (e.g. controlled value drift), start at 0
      if (currentIndex === -1) currentIndex = 0;

      const now = performance.now();
      // Throttle rapid navigation to avoid quick back-and-forth when async updates race
      if (now - lastNavRef.current < 60) return;

      if (event.key === 'ArrowDown') {
        currentIndex = (currentIndex + 1) % options.length;
      } else if (event.key === 'ArrowUp') {
        currentIndex = (currentIndex - 1 + options.length) % options.length;
      }

      const nextOption = options[currentIndex];
      if (!isValidElement<OptionProps<T>>(nextOption) || nextOption.type !== Option) return;
      const nextValue = (nextOption.props as OptionProps<T>).value;
      if (nextValue !== undefined) {
        lastNavRef.current = now;
        onItemChange(nextValue);
      }
    };

    document.addEventListener('keydown', handleArrowNavigation);
    return () => document.removeEventListener('keydown', handleArrowNavigation);
  }, [onItemChange, children, currentValue]);

  return (
    <SelectContext.Provider
      value={{
        rect:
          wrapperRef.current?.getBoundingClientRect() ??
          (typeof window !== 'undefined'
            ? new DOMRect()
            : ({ left: 0, top: 0, width: 0, height: 0, right: 0, bottom: 0 } as DOMRect)),
        element: wrapperRef.current,
        value: currentValue,
        label: label ?? '',
        close: () => setIsOpen(false),
      }}
    >
      <div className={cn('w-full', className)}>
        <button
          disabled={disabled}
          onClick={toggle}
          ref={wrapperRef}
          tabIndex={0}
          type="button"
          className={cn('relative w-full rounded-md', focusable)}
        >
          {/* Active item */}
          <div
            className={cn(baseClasses, {
              'text-disabled cursor-not-allowed bg-neutral-500': disabled,
            })}
          >
            <div>
              {/* Placeholder */}
              {!selectedOption && <div className="text-gray">{placeholder ? placeholder : 'Select an option'}</div>}
              {selectedOption && isValidElement(selectedOption) && <div>{label}</div>}
            </div>
            <div>
              {isOpen && <FontAwesomeIcon icon={faChevronUp} />}
              {!isOpen && <FontAwesomeIcon icon={faChevronDown} />}
            </div>
          </div>
          {isOpen && <SelectOverlay onChange={onItemChange} children={children} />}
        </button>
      </div>
    </SelectContext.Provider>
  );
}

function SelectOverlay<T>({ children, onChange }: { children: React.ReactNode; onChange?: (value: T) => void }) {
  const selectContext = useContext(SelectContext);
  const overlayRef = useRef<HTMLDivElement>(null);
  const isOpen = useRef(false);
  const position = useOverlayPosition();

  const overlayChildren = useMemo(
    () =>
      Children.toArray(children).map(child => {
        if (!isValidElement<OptionProps<T>>(child) || child.type !== Option) return null;
        if (child.props.type === 'separator') return cloneElement(child, { ...child.props });
        return cloneElement(child, {
          ...child.props,
          selected: child.props.value === selectContext.value,
          onChange: () => onChange?.(child.props.value as T),
        });
      }),
    [],
  );

  // Close the overlay when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      event.stopPropagation();
      event.preventDefault();
      if (
        overlayRef.current &&
        !overlayRef.current.contains(event.target as Node) &&
        selectContext.element &&
        !selectContext.element.contains(event.target as Node)
      ) {
        selectContext.close?.();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [selectContext]);

  // Handle keyboard navigation
  useEffect(() => {
    const handleArrowNavigation = (event: KeyboardEvent) => {
      if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
      event.preventDefault();
      const options = overlayRef.current?.querySelectorAll('[role="option"]');
      if (!options || options.length === 0) return;

      let currentIndex = Array.from(options).findIndex(option => option.getAttribute('aria-selected') === 'true');

      if (event.key === 'ArrowDown') {
        currentIndex = (currentIndex + 1) % options.length;
      } else if (event.key === 'ArrowUp') {
        currentIndex = (currentIndex - 1 + options.length) % options.length;
      }

      const nextOption = options[currentIndex] as HTMLElement;
      nextOption.focus();
    };

    document.addEventListener('keydown', handleArrowNavigation);
    return () => document.removeEventListener('keydown', handleArrowNavigation);
  }, []);

  useLayoutEffect(() => {
    if (!overlayRef.current || isOpen.current || !selectContext.element) return;

    // Set the overlay width to match the select width
    overlayRef.current.style.width = `${selectContext.rect.width}px`;

    // Calculate and set the position of the overlay
    const pos = position(selectContext.element, overlayRef, 'below', 'center');

    // Apply the calculated position to the overlay
    overlayRef.current.style.left = `${pos.left}px`;
    overlayRef.current.style.top = `${pos.top}px`;

    const child = overlayChildren.find(child => child?.props.selected);
    child && overlayRef.current.querySelector(`[aria-selected]`)?.scrollIntoView({ block: 'nearest' });
  }, [overlayChildren]);

  const overlayDiv = (
    <div ref={overlayRef} className={cn('absolute z-10 max-h-125 overflow-auto rounded-sm bg-white')}>
      {overlayChildren}
    </div>
  );

  const backdropDiv = <div className="fixed inset-0 z-5 h-screen w-screen" aria-hidden="true" />;

  if (!selectContext.element || !selectContext.element.parentElement) return null;
  return (
    <>
      {/* The container for the dropdown with the options */}
      {createPortal(overlayDiv, selectContext.element.parentElement)}
      {/* The backdrop to catch outside clicks */}
      {createPortal(backdropDiv, document.body)}
    </>
  );
}
