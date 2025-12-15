'use client';

import { cn, focusable } from '@/client/lib/util';
import { ReactNode, useCallback } from 'react';

export interface OptionValue<T> {
  value: T;
  type?: never;
  title?: never;
  children: ReactNode;
  selected?: boolean;
  onChange?: (value: T) => void;
}

export interface OptionType {
  value?: never;
  type: 'separator';
  title?: string;
  children?: never;
  selected?: never;
  onChange?: never;
}

export type OptionProps<T> = OptionValue<T> | OptionType;

export function Option<T>({ value, type, title, children, onChange, selected }: OptionProps<T>) {
  const handleClick = useCallback(() => onChange?.(value!), [value]);

  if (type === 'separator')
    return (
      <div role="separator" className="relative my-2">
        <div role="separator" className="h-px bg-gray-300" />
        <span className="text-default absolute -mt-2 ml-2 bg-white px-2 text-xs">{title}</span>
      </div>
    );

  return (
    <button
      data-slot="option"
      type="button"
      role="option"
      className={cn(
        'hover:bg-hover aria-[selected]:bg-active text-default block w-full cursor-pointer px-4 py-2 text-left',
        focusable,
      )}
      onClick={handleClick}
      aria-selected={selected || undefined}
    >
      {children}
    </button>
  );
}
Option.displayName = 'Option';
