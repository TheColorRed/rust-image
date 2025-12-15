'use client';

import { useOverlayPosition } from '@/client/hooks/overlay-position';
import { cn, focusable } from '@/client/lib/util';
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
  value: v,
  className,
  disabled,
  children,
  placeholder,
  size = 'md',
  onSelect,
}: SelectProps<T>) {
  const wrapperRef = useRef<HTMLButtonElement>(null);
  const [value, setValue] = useState<T | undefined>(v);
  const hiddenValueRef = useRef<HTMLInputElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  // const isOpen = useRef(false);
  // const toggle = () => (isOpen.current = !isOpen.current);
  const toggle = () => setIsOpen(v => !v);

  // Find the selected option
  const selectedOption = useMemo(
    () => Children.toArray(children).find(child => isValidElement<{ value: T }>(child) && child.props.value === value),
    [children, value],
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
    setValue(v as T);
  }, [v]);

  const label = useMemo(() => {
    if (React.isValidElement<{ children: ReactNode }>(selectedOption) && 'props' in selectedOption) {
      return selectedOption.props.children;
    }
    return undefined;
  }, [selectedOption]);

  const onItemChange = useCallback(
    (v: T) => {
      setValue(v);
      onSelect?.(v);
    },
    [hiddenValueRef, setValue, onSelect],
  );

  return (
    <SelectContext.Provider
      value={{
        rect:
          wrapperRef.current?.getBoundingClientRect() ??
          (typeof window !== 'undefined'
            ? new DOMRect()
            : ({ left: 0, top: 0, width: 0, height: 0, right: 0, bottom: 0 } as DOMRect)),
        element: wrapperRef.current,
        value,
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

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
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

  useLayoutEffect(() => {
    if (!overlayRef.current || isOpen.current || !selectContext.element) return;

    // Set the overlay width to match the select width
    overlayRef.current.style.width = `${selectContext.rect.width}px`;

    // Calculate and set the position of the overlay
    const pos = position(selectContext.element, overlayRef, 'below', 'center');

    // Apply the calculated position to the overlay
    overlayRef.current.style.left = `${pos.left}px`;
    overlayRef.current.style.top = `${pos.top}px`;
  }, []);

  const overlayChildren = useMemo(
    () =>
      Children.toArray(children).map(child => {
        if (!isValidElement<OptionProps<T>>(child) || child.type !== Option) return null;
        if (child.props.type === 'separator') return cloneElement(child, { ...child.props });
        return cloneElement(child, {
          ...child.props,
          onChange: () => onChange?.(child.props.value as T),
        });
      }),
    [],
  );

  const overlayDiv = (
    <div ref={overlayRef} className={cn('absolute z-10 max-h-[500px] overflow-auto rounded-sm bg-white')}>
      {overlayChildren}
    </div>
  );

  if (!selectContext.element || !selectContext.element.parentElement) return null;
  return createPortal(overlayDiv, selectContext.element.parentElement);
}
