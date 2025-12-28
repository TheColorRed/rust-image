'use client';

import { cn, focusable } from '@/lib/util';
import { clsx } from 'clsx';
import { cloneElement, createElement, isValidElement, useCallback, useEffect, useMemo, useRef } from 'react';
import { twMerge } from 'tailwind-merge';

export interface TextareaProps {
  value?: string | number;
  placeholder?: string;
  name?: string;
  required?: boolean;
  rows?: number;
  cols?: number;
  className?: string;
  autoHeight?: boolean;
  disabled?: boolean;
  maxLength?: number;
  id?: string;
  autocomplete?: string;
  ref?: React.Ref<HTMLTextAreaElement | HTMLInputElement>;
  onChange?: (value: string) => void;
  onFocus?: (e: React.FocusEvent<HTMLTextAreaElement | HTMLInputElement>) => void;
  onBlur?: (value: string) => void;
  onKeyUp?: (e: React.KeyboardEvent<HTMLTextAreaElement | HTMLInputElement>) => void;
  onKeyDown?: (e: React.KeyboardEvent<HTMLTextAreaElement | HTMLInputElement>) => void;
}

export type InputProps = Omit<TextareaProps, 'autoHeight' | 'rows' | 'cols'> & {
  suffix?: string | React.ReactNode | React.FunctionComponent<React.SVGProps<SVGSVGElement>>;
  prefix?: string | React.ReactNode | React.FunctionComponent<React.SVGProps<SVGSVGElement>>;
  max?: number;
  min?: number;
  /**
   * If true, selects the input text when focused.
   */
  selectFocus?: boolean;
  /**
   * If true, allows incrementing/decrementing numeric input values using the mouse scroll wheel.
   */
  scrollWheelIncrement?: boolean;
};

const disabledClasses = 'disabled:cursor-not-allowed disabled:bg-neutral-500 disabled:text-disabled';
const baseClasses = clsx(
  // Sizing
  'w-full',
  // Border
  'border border-gray-300 bg-gray-100 rounded-md py-1 px-3 text-gray',
  'text-neutral-300 bg-medium',
  focusable,
);

export function Textarea({
  value,
  placeholder,
  className,
  maxLength = undefined,
  autoHeight = false,
  disabled = false,
  name,
  id = undefined,
  required = false,
  rows = 3,
  cols = 20,
  autocomplete = undefined,
  onChange,
  onKeyUp,
  onFocus,
  onKeyDown,
  onBlur,
  ref,
}: TextareaProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const classes = twMerge(
    clsx(baseClasses, 'resize-none', {
      'overflow-hidden': autoHeight,
      [disabledClasses]: disabled,
    }),
    className,
  );

  const onInput = useCallback(() => {
    if (!autoHeight) return;
    const textarea = textareaRef.current;
    if (!textarea) return;

    textarea.style.height = 'auto';
    textarea.style.height = `${textarea.scrollHeight}px`;
  }, [autoHeight]);

  const setRef = useCallback((node: HTMLTextAreaElement | null) => {
    if (ref && typeof ref === 'function') ref(node);
    else if (ref && typeof ref === 'object' && ref !== null)
      (ref as React.RefObject<HTMLTextAreaElement | HTMLInputElement | null>).current = node;
    textareaRef.current = node;
  }, []);

  return (
    <textarea
      ref={setRef}
      value={value ?? ''}
      placeholder={placeholder}
      onChange={e => onChange?.(String(e.target.value))}
      onInput={onInput}
      onKeyUp={onKeyUp}
      onKeyDown={onKeyDown}
      onFocus={onFocus}
      onBlur={e => onBlur?.(String(e.target.value))}
      disabled={disabled}
      maxLength={maxLength}
      className={classes}
      name={name}
      rows={rows}
      cols={cols}
      id={id}
      autoComplete={autocomplete}
      required={required}
    />
  );
}

export function Input(
  props: Omit<React.ComponentProps<'input'>, 'onChange' | 'onBlur' | 'prefix'> &
    Omit<InputProps, 'ref'> & { ref?: React.Ref<HTMLInputElement> },
) {
  const inputRef = useRef<HTMLInputElement>(null);
  const setRef = useCallback(
    (node: HTMLInputElement | null) => {
      const { ref } = props;
      if (ref && typeof ref === 'function') ref(node);
      else if (ref && typeof ref === 'object' && ref !== null)
        (ref as React.RefObject<HTMLInputElement | null>).current = node;
      inputRef.current = node;
    },
    [props.ref],
  );

  const inputProps = Object.entries(props).reduce<{ [key: string]: any }>((acc, [key, value]) => {
    const filtered = ['ref', 'onChange', 'suffix', 'prefix', 'selectFocus', 'scrollWheelIncrement'].includes(key);
    if (filtered) return acc;
    acc[key] = value;
    return acc;
  }, {} as React.ComponentProps<'input'>);

  useEffect(() => {
    const input = inputRef.current;
    if (!input || !props.scrollWheelIncrement) return;

    const handleWheel = (e: WheelEvent) => {
      e.preventDefault();
      const step = Number(input.step) || 1;
      const currentValue = Number(input.value) || 0;
      const delta = Math.sign(e.deltaY) * -1; // Invert to match natural scrolling
      let newValue = currentValue + delta * step;
      if (props.min !== undefined) newValue = Math.max(newValue, props.min);
      if (props.max !== undefined) newValue = Math.min(newValue, props.max);
      input.value = String(newValue);
      props.onChange?.(String(newValue));
    };

    input.addEventListener('wheel', handleWheel, { passive: false });

    return () => {
      input.removeEventListener('wheel', handleWheel);
    };
  }, [props.scrollWheelIncrement, props.min, props.max, props.onChange]);

  const input = useMemo(
    () => (
      <input
        {...inputProps}
        data-slot="input"
        ref={setRef}
        onChange={e => props.onChange?.(String(e.target.value))}
        onBlur={e => props.onBlur?.(String(e.target.value))}
        className={cn(baseClasses, {
          [disabledClasses]: props.disabled,
        })}
        onFocus={e => {
          if (props.selectFocus) e.target.select();
          props.onFocus?.(e as React.FocusEvent<HTMLInputElement>);
        }}
      />
    ),
    [props.disabled, props.onChange, inputProps, setRef],
  );

  const suffix = props.suffix;
  const prefix = props.prefix;
  const className = props.className;
  return (
    <div className={cn('relative w-full', className)} onClick={() => inputRef.current?.focus()}>
      {(suffix || prefix) && (
        <div>
          <span
            className={cn('absolute top-1/2 -translate-y-1/2 text-slate-400 [&_svg]:h-4 [&_svg]:w-4', {
              'right-3': typeof suffix === 'string' || isValidElement(suffix),
              'left-3': typeof prefix === 'string' || isValidElement(prefix),
            })}
          >
            {typeof suffix === 'string' ? suffix : isValidElement(suffix) ? cloneElement(suffix) : null}
            {typeof prefix === 'string' ? prefix : isValidElement(prefix) ? cloneElement(prefix) : null}
          </span>
          {createElement(input.type, {
            ...input.props,
            // Resize the input so it doesn't overlap the suffix
            className: cn(input.props.className, 'pr-8', {
              'pr-[2.5rem]': typeof suffix === 'string' || isValidElement(suffix),
              'pl-[2.5rem]': typeof prefix === 'string' || isValidElement(prefix),
            }),
          })}
        </div>
      )}
      {!suffix && !prefix && input}
    </div>
  );
}
