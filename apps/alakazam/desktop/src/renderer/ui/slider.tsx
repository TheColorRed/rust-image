'use client';

import { cn, focusable } from '@/lib/util';
import { MouseEvent, useCallback, useEffect, useRef, useState } from 'react';

export interface SliderProps {
  /** The minimum value of the range. */
  min?: number;
  /** The maximum value of the range. */
  max?: number;
  /** The current value of the range. */
  value?: number;
  /** The step value for the range. */
  step?: number;
  /**
   * If true, the slider will snap back to the initial value when close to it while dragging.
   * This is useful for adjustments that have a neutral starting point.
   */
  snapAtStart?: boolean;
  /** Callback when the range value changes. */
  onChange?: (value: number) => void;
  /** Callback when the range value when dragging the handles. */
  onDrag?: (value: number) => void;
}

export default function Slider({
  min = 0,
  max = 100,
  value = 0,
  step = 1,
  snapAtStart = false,
  onChange,
  onDrag,
}: SliderProps) {
  const [cValue, setCValue] = useState(value);
  const [displayValue, setDisplayValue] = useState(value);
  const [isDragging, setIsDragging] = useState(false);
  const wrapperRef = useRef<HTMLDivElement>(null);
  const isDraggingRef = useRef(false);
  const currentValue = useRef(cValue);
  const handleRef = useRef<HTMLDivElement>(null);
  const initialStartValue = useRef<number>(value);

  useEffect(() => {
    const v = Math.min(Math.max(min, value), max);
    currentValue.current = v;
    setDisplayValue(v);
    setCValue(v);
  }, [value, min, max]);

  const changeValue = useCallback(
    (value: number) => {
      const snapToStep = (value: number) => Math.round((value - min) / step) * step + min;
      // const x = handleX - wrapperRect.left;
      // const rawValue = (x / wrapperRect.width) * max;
      const newValue = Math.min(Math.max(min, snapToStep(value)), max);
      currentValue.current = newValue;
      onDrag?.(currentValue.current);
      setDisplayValue(currentValue.current);
    },
    [min, max, step, onDrag, onChange],
  );

  const handleMouseDown = useCallback(
    (e: MouseEvent<HTMLDivElement>) => {
      e.preventDefault();
      const wrapperRect = wrapperRef.current?.getBoundingClientRect();
      if (!wrapperRect) return;
      const rawX = e.clientX - wrapperRect.left;
      const x = Math.min(Math.max(0, rawX), wrapperRect.width);
      const rawValue = min + (x / wrapperRect.width) * (max - min);
      changeValue(rawValue);
      setCValue(currentValue.current);
      onChange?.(currentValue.current);
      isDraggingRef.current = true;
      document.body.style.userSelect = 'none';
      setIsDragging(true);
    },
    [min, max, changeValue, onChange],
  );

  const handleMouseUp = useCallback(() => {
    if (isDraggingRef.current) {
      setCValue(currentValue.current);
      onChange?.(currentValue.current);
    }
    isDraggingRef.current = false;
    setIsDragging(false);
    document.body.style.userSelect = '';
  }, [onChange]);

  const mouseMoveHandler = useCallback(
    (e: globalThis.MouseEvent) => {
      if (isDraggingRef.current) {
        const wrapperRect = wrapperRef.current?.getBoundingClientRect();
        if (!wrapperRect) return;
        const rawX = e.clientX - wrapperRect.left;
        const x = Math.min(Math.max(0, rawX), wrapperRect.width);
        const rawValue = min + (x / wrapperRect.width) * (max - min);

        // Apply snapping logic if enabled
        let newValue = rawValue;
        if (snapAtStart) {
          const snapThreshold = (max - min) * 0.025; // 2.5% of the range for snapping
          if (Math.abs(rawValue - initialStartValue.current) <= snapThreshold) {
            newValue = initialStartValue.current;
          }
        }

        changeValue(newValue);
        setCValue(currentValue.current);
      }
    },
    [isDraggingRef, min, max, step, currentValue, onDrag, snapAtStart],
  );

  const handleKeyPress = useCallback(
    (e: KeyboardEvent) => {
      if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) return;
      if (handleRef.current !== document.activeElement) return;
      e.preventDefault();

      const currentStep = e.ctrlKey ? step * 20 : e.altKey ? step * 10 : step;

      if (['ArrowLeft', 'ArrowDown'].includes(e.key)) {
        changeValue(currentValue.current - currentStep);
        setCValue(currentValue.current - currentStep);
      } else if (['ArrowRight', 'ArrowUp'].includes(e.key)) {
        changeValue(currentValue.current + currentStep);
        setCValue(currentValue.current + currentStep);
      }
    },
    [step, cValue, value],
  );

  useEffect(() => {
    window.addEventListener('mousemove', mouseMoveHandler);
    window.addEventListener('mouseup', handleMouseUp);
    window.addEventListener('keydown', handleKeyPress);
    return () => {
      window.removeEventListener('mousemove', mouseMoveHandler);
      window.removeEventListener('mouseup', handleMouseUp);
      window.removeEventListener('keydown', handleKeyPress);
    };
  }, [mouseMoveHandler, handleMouseUp, handleKeyPress]);

  return (
    <div className="relative w-full cursor-pointer" ref={wrapperRef} onMouseDown={handleMouseDown}>
      {/* Vertical Bar */}
      {snapAtStart && (
        <div
          className="bg-muted absolute -top-3 h-7.5 w-0.5"
          style={{
            left: `calc(${((initialStartValue.current - min) / (max - min)) * 100}% - 2.5px)`,
          }}
        />
      )}
      {/* Bar */}
      <div className="bg-muted h-2 w-full rounded-md" />
      {/* Range */}
      <div
        className="bg-primary absolute top-0 left-0 h-2 rounded-md"
        style={{
          width: `${((currentValue.current - min) / (max - min)) * 100}%`,
          left: `calc(${((min - min) / (max - min)) * 100}%)`,
        }}
      />

      {/* Handle */}
      <div
        ref={handleRef}
        className={cn(
          'bg-primary absolute h-4 w-4 cursor-grab rounded-full',
          focusable,
          isDragging && 'cursor-grabbing',
        )}
        style={{
          left: `calc(${((displayValue - min) / (max - min)) * 100}% - 8px)`,
          top: '-3px',
        }}
        tabIndex={0}
        data-name="handle"
      />
    </div>
  );
}
